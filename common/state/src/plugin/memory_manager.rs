use std::{
    mem::MaybeUninit,
    sync::atomic::{AtomicPtr, Ordering},
};

use serde::{Deserialize, Serialize};
use specs::{
    storage::GenericReadStorage, Component, Entities, Entity, Read, ReadStorage, WriteStorage,
};
use wasmer::{Memory, StoreMut, StoreRef, TypedFunction, WasmPtr};

use common::{
    comp::{Health, Player},
    uid::{IdMaps, Uid},
};

use super::{
    errors::{MemoryAllocationError, PluginModuleError},
    MemoryModel,
};

pub struct EcsWorld<'a, 'b> {
    pub entities: &'b Entities<'a>,
    pub health: EcsComponentAccess<'a, 'b, Health>,
    pub uid: EcsComponentAccess<'a, 'b, Uid>,
    pub player: EcsComponentAccess<'a, 'b, Player>,
    pub id_maps: &'b Read<'a, IdMaps>,
}

pub enum EcsComponentAccess<'a, 'b, T: Component> {
    Read(&'b ReadStorage<'a, T>),
    ReadOwned(ReadStorage<'a, T>),
    Write(&'b WriteStorage<'a, T>),
    WriteOwned(WriteStorage<'a, T>),
}

impl<'a, 'b, T: Component> EcsComponentAccess<'a, 'b, T> {
    pub fn get(&self, entity: Entity) -> Option<&T> {
        match self {
            EcsComponentAccess::Read(e) => e.get(entity),
            EcsComponentAccess::Write(e) => e.get(entity),
            EcsComponentAccess::ReadOwned(e) => e.get(entity),
            EcsComponentAccess::WriteOwned(e) => e.get(entity),
        }
    }
}

impl<'a, 'b, T: Component> From<&'b ReadStorage<'a, T>> for EcsComponentAccess<'a, 'b, T> {
    fn from(a: &'b ReadStorage<'a, T>) -> Self { Self::Read(a) }
}

impl<'a, 'b, T: Component> From<ReadStorage<'a, T>> for EcsComponentAccess<'a, 'b, T> {
    fn from(a: ReadStorage<'a, T>) -> Self { Self::ReadOwned(a) }
}

impl<'a, 'b, T: Component> From<&'b WriteStorage<'a, T>> for EcsComponentAccess<'a, 'b, T> {
    fn from(a: &'b WriteStorage<'a, T>) -> Self { Self::Write(a) }
}

impl<'a, 'b, T: Component> From<WriteStorage<'a, T>> for EcsComponentAccess<'a, 'b, T> {
    fn from(a: WriteStorage<'a, T>) -> Self { Self::WriteOwned(a) }
}

/// This structure wraps the ECS pointer to ensure safety
pub struct EcsAccessManager {
    ecs_pointer: AtomicPtr<EcsWorld<'static, 'static>>,
}

impl Default for EcsAccessManager {
    fn default() -> Self {
        Self {
            ecs_pointer: AtomicPtr::new(std::ptr::null_mut()),
        }
    }
}

impl EcsAccessManager {
    // This function take a World reference and a function to execute ensuring the
    // pointer will never be corrupted during the execution of the function!
    pub fn execute_with<T>(&self, world: &EcsWorld, func: impl FnOnce() -> T) -> T {
        let _guard = scopeguard::guard((), |_| {
            // ensure the pointer is cleared in any case
            self.ecs_pointer
                .store(std::ptr::null_mut(), Ordering::Relaxed);
        });
        self.ecs_pointer
            .store(world as *const _ as *mut _, Ordering::Relaxed);
        func()
    }

    /// This unsafe function returns a reference to the Ecs World
    ///
    /// # Safety
    /// This function is safe to use if it matches the following requirements
    ///  - The reference and subreferences like Entities, Components ... aren't
    ///    leaked out the thread
    ///  - The reference and subreferences lifetime doesn't exceed the source
    ///    function lifetime
    ///  - Always safe when called from `retrieve_action` if you don't pass a
    ///    reference somewhere else
    ///  - All that ensure that the reference doesn't exceed the execute_with
    ///    function scope
    pub unsafe fn get(&self) -> Option<&EcsWorld> {
        // ptr::as_ref will automatically check for null
        self.ecs_pointer.load(Ordering::Relaxed).as_ref()
    }
}

/// This function check if the buffer is wide enough if not it realloc the
/// buffer calling the `wasm_prepare_buffer` function Note: There is
/// probably optimizations that can be done using less restrictive
/// ordering
fn get_pointer(
    store: &mut StoreMut,
    object_length: <MemoryModel as wasmer::MemorySize>::Offset,
    allocator: &TypedFunction<
        <MemoryModel as wasmer::MemorySize>::Offset,
        WasmPtr<u8, MemoryModel>,
    >,
) -> Result<WasmPtr<u8, MemoryModel>, MemoryAllocationError> {
    allocator
        .call(store, object_length)
        .map_err(MemoryAllocationError::CantAllocate)
}

/// This functions wraps the serialization process
fn serialize_data<T: Serialize>(object: &T) -> Result<Vec<u8>, PluginModuleError> {
    bincode::serialize(object).map_err(PluginModuleError::Encoding)
}

/// This function writes an object to the wasm memory using the allocator if
/// necessary using length padding.
///
/// With length padding the first bytes written are the length of the the
/// following slice (The object serialized).
pub(crate) fn write_serialized_with_length<T: Serialize>(
    store: &mut StoreMut,
    memory: &Memory,
    allocator: &TypedFunction<
        <MemoryModel as wasmer::MemorySize>::Offset,
        WasmPtr<u8, MemoryModel>,
    >,
    object: &T,
) -> Result<WasmPtr<u8, MemoryModel>, PluginModuleError> {
    write_length_and_bytes(store, memory, allocator, &serialize_data(object)?)
}

/// This function writes an raw bytes to WASM memory returning a pointer and
/// a length. Will realloc the buffer is not wide enough.
///
/// As this function is often called after prepending a length to an existing
/// object it accepts two slices and concatenates them to cut down copying in
/// the caller.
pub(crate) fn write_bytes(
    store: &mut StoreMut,
    memory: &Memory,
    allocator: &TypedFunction<
        <MemoryModel as wasmer::MemorySize>::Offset,
        WasmPtr<u8, MemoryModel>,
    >,
    bytes: (&[u8], &[u8]),
) -> Result<
    (
        WasmPtr<u8, MemoryModel>,
        <MemoryModel as wasmer::MemorySize>::Offset,
    ),
    PluginModuleError,
> {
    let len = (bytes.0.len() + bytes.1.len()) as <MemoryModel as wasmer::MemorySize>::Offset;
    let ptr = get_pointer(store, len, allocator).map_err(PluginModuleError::MemoryAllocation)?;
    ptr.slice(
        &memory.view(store),
        len as <MemoryModel as wasmer::MemorySize>::Offset,
    )
    .and_then(|s| {
        s.subslice(0..bytes.0.len() as u64).write_slice(bytes.0)?;
        s.subslice(bytes.0.len() as u64..len).write_slice(bytes.1)
    })
    .map_err(|_| PluginModuleError::InvalidPointer)?;
    Ok((ptr, len))
}

/// This function writes bytes to the wasm memory using the allocator if
/// necessary using length padding.
///
/// With length padding the first bytes written are the length of the the
/// following slice.
pub(crate) fn write_length_and_bytes(
    store: &mut StoreMut,
    memory: &Memory,
    allocator: &TypedFunction<
        <MemoryModel as wasmer::MemorySize>::Offset,
        WasmPtr<u8, MemoryModel>,
    >,
    bytes: &[u8],
) -> Result<WasmPtr<u8, MemoryModel>, PluginModuleError> {
    let len = bytes.len() as <MemoryModel as wasmer::MemorySize>::Offset;
    write_bytes(store, memory, allocator, (&len.to_le_bytes(), bytes)).map(|val| val.0)
}

/// This function reads data from memory at a position with the array length and
/// converts it to an object using bincode
pub(crate) fn read_serialized<'a, T: for<'b> Deserialize<'b>>(
    memory: &'a Memory,
    store: &StoreRef,
    ptr: WasmPtr<u8, MemoryModel>,
    len: <MemoryModel as wasmer::MemorySize>::Offset,
) -> Result<T, bincode::Error> {
    bincode::deserialize(
        &read_bytes(memory, store, ptr, len).map_err(|_| bincode::ErrorKind::SizeLimit)?,
    )
}

/// This function reads raw bytes from memory at a position with the array
/// length
pub(crate) fn read_bytes(
    memory: &Memory,
    store: &StoreRef,
    ptr: WasmPtr<u8, MemoryModel>,
    len: <MemoryModel as wasmer::MemorySize>::Offset,
) -> Result<Vec<u8>, PluginModuleError> {
    ptr.slice(&memory.view(store), len)
        .and_then(|s| s.read_to_vec())
        .map_err(|_| PluginModuleError::InvalidPointer)
}

/// This function reads a constant amount of raw bytes from memory
pub(crate) fn read_exact_bytes<const N: usize>(
    memory: &Memory,
    store: &StoreRef,
    ptr: WasmPtr<u8, MemoryModel>,
) -> Result<[u8; N], PluginModuleError> {
    let mut result = MaybeUninit::uninit_array();
    ptr.slice(&memory.view(store), N.try_into().unwrap())
        .and_then(|s| s.read_slice_uninit(&mut result))
        .map_err(|_| PluginModuleError::InvalidPointer)?;
    unsafe { Ok(MaybeUninit::array_assume_init(result)) }
}
