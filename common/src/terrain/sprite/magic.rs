#[macro_export]
macro_rules! sprites {
    (
        $($category_name:ident = $category_disc:literal $(has $($attr:ident),* $(,)?)? {
            $($sprite_name:ident = $sprite_id:literal),* $(,)?
        }),* $(,)?
    ) => {
        make_case_elim!(
            category,
            #[derive(Copy, Clone, Debug, Hash, Eq, PartialEq, Serialize, Deserialize, EnumIter, FromPrimitive)]
            #[repr(u32)]
            pub enum Category {
                $($category_name = $category_disc,)*
            }
        );

        impl Category {
            #[inline] pub const fn all() -> &'static [Self] {
                &[$(Self::$category_name,)*]
            }

            #[cfg(test)]
            #[inline] const fn all_sprites(&self) -> &'static [SpriteKind] {
                match self {
                    $(Self::$category_name => &[$(SpriteKind::$sprite_name,)*],)*
                }
            }

            // Size, in bits, of the sprite ID
            #[inline] pub const fn sprite_id_mask(&self) -> u32 {
                match self {
                    $(Self::$category_name => ((0u32 $(| $sprite_id)*) + 1).next_power_of_two() - 1,)*
                }
            }

            // Size, in bits, of the sprite ID
            #[inline] pub const fn sprite_id_size(&self) -> u32 { self.sprite_id_mask().count_ones() }

            // The mask that, when applied to the block data, yields the sprite kind
            #[inline(always)] pub const fn sprite_kind_mask(&self) -> u32 { 0x00FF0000 | self.sprite_id_mask() }

            /// Note that this function assumes that the `BlockKind` of `block` permits sprite inhabitants
            /// (i.e: is unfilled).
            #[allow(non_upper_case_globals)]
            #[inline] pub(super) const fn from_block(block: Block) -> Option<Self> {
                $(const $category_name: u8 = Category::$category_name as u8;)*
                match block.sprite_category_byte() {
                    $($category_name => Some(Self::$category_name),)*
                    _ => None,
                }
            }

            // TODO: It would be nice to use `NonZeroU8` here for the space saving, but `0` is a valid
            // offset for categories with only one SpriteKind (i.e: the sprite ID is zero-length and so
            // attributes can go right up to the end of the block data). However, we could decide that an
            // offset of, say, 0xFF (which would obviously be far out of bounds anyway) represents 'this
            // attribute has no presence in this category'.
            #[inline] pub const fn attr_offsets(&self) -> &[Option<u8>; Attributes::all().len()] {
                match self {
                    $(Self::$category_name => {
                        #[allow(unused_mut, unused_variables, unused_assignments)]
                        const fn gen_attr_offsets() -> [Option<u8>; Attributes::all().len()] {
                            let mut lut = [None; Attributes::all().len()];
                            // Don't take up space used by the sprite ID
                            let mut offset = Category::$category_name.sprite_id_size();
                            $($({
                                // Perform basic checks
                                if offset + $attr::BITS as u32 > 16 {
                                    panic!("Sprite category has an attribute set that will not fit in the block data");
                                } else if lut[$attr::INDEX].is_some() {
                                    panic!("Sprite category cannot have more than one instance of an attribute");
                                } else if offset > (!0u8) as u32 {
                                    panic!("Uhhh");
                                }
                                lut[$attr::INDEX] = Some(offset as u8);
                                offset += $attr::BITS as u32;
                            })*)*
                            lut
                        }
                        const ATTR_OFFSETS: [Option<u8>; Attributes::all().len()] = gen_attr_offsets();
                        &ATTR_OFFSETS
                    },)*
                }
            }

            /// Returns `true` if this category of sprite has the given attribute.
            #[inline] pub fn has_attr<A: Attribute>(&self) -> bool {
                self.attr_offsets()[A::INDEX].is_some()
            }

            /// Read an attribute from the given block.
            ///
            /// Note that this function assumes that the category of `self` matches that of the block, but does
            /// not validate this.
            #[inline] pub(super) fn read_attr<A: Attribute>(&self, block: Block) -> Result<A, AttributeError<A::Error>> {
                let offset = match self.attr_offsets()[A::INDEX] {
                    Some(offset) => offset,
                    None => return Err(AttributeError::NotPresent),
                };
                let bits = (block.to_be_u32() >> offset as u32) & ((1 << A::BITS as u32) - 1);
                A::from_bits(bits as u16).map_err(AttributeError::Attribute)
            }

            /// Write an attribute to the given block.
            ///
            /// Note that this function assumes that the category of `self` matches that of the block, but does
            /// not validate this.
            #[inline] pub(super) fn write_attr<A: Attribute>(&self, block: &mut Block, attr: A) -> Result<(), AttributeError<core::convert::Infallible>> {
                let offset = match self.attr_offsets()[A::INDEX] {
                    Some(offset) => offset,
                    None => return Err(AttributeError::NotPresent),
                };
                let bits = attr.into_bits() as u32;
                #[cfg(debug_assertions)]
                assert!(bits < (1 << A::BITS as u32), "The bit representation of the attribute {} must fit within {} bits, but the representation was {:0b}", core::any::type_name::<A>(), A::BITS, bits);
                let data = ((block.to_be_u32() & (!(((1 << A::BITS as u32) - 1) << offset as u32))) | (bits << offset as u32)).to_be_bytes();
                *block = block.with_data([data[1], data[2], data[3]]);
                Ok(())
            }
        }

        #[inline] const fn gen_discriminant(category: Category, id: u16) -> u32 {
            (category as u32) << 16 | id as u32
        }

        make_case_elim!(
            sprite_kind,
            #[derive(Copy, Clone, Debug, Hash, Eq, PartialEq, Serialize, Deserialize, EnumIter, FromPrimitive)]
            #[repr(u32)]
            pub enum SpriteKind {
                $($($sprite_name = $crate::terrain::sprite::gen_discriminant($crate::terrain::sprite::Category::$category_name, $sprite_id),)*)*
            }
        );

        impl SpriteKind {
            #[inline] pub const fn all() -> &'static [Self] {
                &[$($(Self::$sprite_name,)*)*]
            }

            #[inline] pub const fn category(&self) -> Category {
                match self {
                    $($(Self::$sprite_name => Category::$category_name,)*)*
                }
            }

            /// Note that this function assumes that the category of `self` matches that of the block data, but does
            /// not validate this.
            #[allow(non_upper_case_globals)]
            #[inline] pub(super) const fn from_block(block: Block) -> Option<Self> {
                match block.sprite_category() {
                    None => None,
                    $(Some(category @ Category::$category_name) => {
                        $(const $sprite_name: u32 = SpriteKind::$sprite_name as u32;)*
                        match block.to_be_u32() & category.sprite_kind_mask() {
                            $($sprite_name => Some(Self::$sprite_name),)*
                            _ => None,
                        }
                    },)*
                }
            }

            #[inline] pub(super) fn to_initial_bytes(self) -> [u8; 3] {
                let sprite_bytes = (self as u32).to_be_bytes();
                let block = Block::from_raw(super::BlockKind::Air, [sprite_bytes[1], sprite_bytes[2], sprite_bytes[3]]);
                match self.category() {
                    $(Category::$category_name => block$($(.with_attr($attr::default()).unwrap())*)?,)*
                }
                    .data()
            }
        }
    };
}

#[derive(Debug)]
pub enum AttributeError<E> {
    /// The attribute was not present for the given block data's category.
    NotPresent,
    /// An attribute-specific error occurred when performing extraction.
    Attribute(E),
}

pub trait Attribute: Default + Sized {
    /// The unique index assigned to this attribute, used to index offset
    /// arrays.
    const INDEX: usize;
    /// The number of bits required to represent this attribute.
    const BITS: u8;
    /// The error that might occur when decoding the attribute from bits.
    type Error: core::fmt::Debug;
    fn from_bits(bits: u16) -> Result<Self, Self::Error>;
    fn into_bits(self) -> u16;
}

#[macro_export]
macro_rules! attributes {
    ($(
        $name:ident { bits: $bits:literal, err: $err:path, from: $from_bits:expr, into: $into_bits:expr $(,)? }
    ),* $(,)?) => {
        #[derive(Copy, Clone, Debug)]
        #[repr(u16)]
        pub enum Attributes {
            $($name,)*
        }

        impl Attributes {
            #[inline] pub const fn all() -> &'static [Self] {
                &[$(Self::$name,)*]
            }
        }

        $(
            #[allow(clippy::all)]
            impl Attribute for $name {
                const INDEX: usize = Attributes::$name as usize;
                const BITS: u8 = $bits;
                type Error = $err;
                #[inline(always)] fn from_bits(bits: u16) -> Result<Self, Self::Error> { $from_bits(bits) }
                #[inline(always)] fn into_bits(self) -> u16 { $into_bits(self) }
            }
        )*
    };
}
