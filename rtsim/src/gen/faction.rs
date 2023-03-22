use crate::data::Faction;
use rand::prelude::*;
use world::{IndexRef, World};

impl Faction {
    pub fn generate(_world: &World, _index: IndexRef, rng: &mut impl Rng) -> Self {
        Self {
            leader: None,
            good_or_evil: rng.gen(),
        }
    }
}
