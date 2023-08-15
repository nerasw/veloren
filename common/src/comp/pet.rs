use crate::comp::{body::Body, phys::Mass, quadruped_medium, quadruped_small};
use crossbeam_utils::atomic::AtomicCell;
use specs::{Component, DerefFlaggedStorage};
use std::{num::NonZeroU64, sync::Arc};
use serde::{Deserialize, Serialize};

pub type PetId = AtomicCell<Option<NonZeroU64>>;

// TODO: move to server crate
#[derive(Clone, Debug)]
pub struct Pet {
    database_id: Arc<PetId>,
}

impl Pet {
    /// Not to be used outside of persistence - provides mutable access to the
    /// pet component's database ID which is required to save the pet's data
    /// from the persistence thread.
    #[doc(hidden)]
    pub fn get_database_id(&self) -> Arc<PetId> { Arc::clone(&self.database_id) }

    pub fn new_from_database(database_id: NonZeroU64) -> Self {
        Self {
            database_id: Arc::new(AtomicCell::new(Some(database_id))),
        }
    }
}

impl Default for Pet {
    fn default() -> Self {
        Self {
            database_id: Arc::new(AtomicCell::new(None)),
        }
    }
}

/// Determines whether an entity of a particular body variant is tameable.
pub fn is_tameable(body: &Body) -> bool {
    // Currently only Quadruped animals can be tamed pending further work
    // on the pets feature (allowing larger animals to be tamed will
    // require balance issues to be addressed).
    match body {
        Body::QuadrupedMedium(quad_med) =>
        // NOTE: the reason we ban mammoth from being tameable even though they're
        // agressive anyway, is that UncomfySilence is going to make them
        // peaceful after this MR gets merged. Please, remove this note in your MR,
        // UncomfySilence!
        {
            !matches!(
                quad_med.species,
                quadruped_medium::Species::Catoblepas
                    | quadruped_medium::Species::Mammoth
                    | quadruped_medium::Species::Hirdrasil
            )
        },
        Body::QuadrupedLow(_) | Body::QuadrupedSmall(_) | Body::BirdMedium(_) => true,
        _ => false,
    }
}

pub fn is_mountable(mount: &Body, rider: Option<&Body>) -> bool {
    let is_light_enough =
        |rider: Option<&Body>| -> bool { rider.map_or(false, |b| b.mass() <= Mass(500.0)) };

    match mount {
        Body::Humanoid(_) => matches!(rider, Some(Body::BirdMedium(_))),
        Body::BipedLarge(_) => is_light_enough(rider),
        Body::BirdLarge(_) => is_light_enough(rider),
        Body::QuadrupedMedium(body) => match body.species {
            quadruped_medium::Species::Alpaca
            | quadruped_medium::Species::Antelope
            | quadruped_medium::Species::Bear
            | quadruped_medium::Species::Camel
            | quadruped_medium::Species::Cattle
            | quadruped_medium::Species::Deer
            | quadruped_medium::Species::Donkey
            | quadruped_medium::Species::Highland
            | quadruped_medium::Species::Horse
            | quadruped_medium::Species::Kelpie
            | quadruped_medium::Species::Llama
            | quadruped_medium::Species::Moose
            | quadruped_medium::Species::Tuskram
            | quadruped_medium::Species::Yak
            | quadruped_medium::Species::Zebra
            | quadruped_medium::Species::Grolgar
            | quadruped_medium::Species::Wolf
            | quadruped_medium::Species::Saber
            | quadruped_medium::Species::Tiger => true,
            quadruped_medium::Species::Mouflon => is_light_enough(rider),
            _ => false,
        },
        Body::QuadrupedSmall(body) => match body.species {
            quadruped_small::Species::Truffler => true,
            quadruped_small::Species::Boar | quadruped_small::Species::Holladon => {
                is_light_enough(rider)
            },
            _ => false,
        },
        Body::QuadrupedLow(_) => mount.mass() >= Mass(300.0) && is_light_enough(rider),
        Body::Ship(_) => true,
        _ => false,
    }
}

impl Component for Pet {
    // Using `DenseVecStorage` has a u64 space overhead per entity and `Pet` just
    // has an `Arc` pointer which is the same size on 64-bit platforms. So it
    // isn't worth using `DenseVecStorage` here.
    type Storage = specs::VecStorage<Self>;
}


#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum StayFollow{
    Stay,
    Follow,
}

#[derive(Copy, Clone, Debug, PartialEq,  Serialize, Deserialize)]
pub struct PetState{
    pub stay: bool,
}

impl Default for PetState {
    fn default() -> Self {
        Self {
            stay: false,
        }
    }
}

impl PetState {
    pub fn get_state(&self) -> bool{
        self.stay
    }
}

impl Component for PetState {
    type Storage = DerefFlaggedStorage<Self>;
}
