use crate::{
    combat::{Attack, AttackSource},
    uid::Uid,
};
use serde::{Deserialize, Serialize};
use specs::{Component, DerefFlaggedStorage};
use std::time::Duration;

#[derive(Copy, Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum ShockwaveDodgeable {
    Roll,
    Jump,
    No,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Properties {
    pub angle: f32,
    pub vertical_angle: f32,
    pub speed: f32,
    pub attack: Attack,
    pub dodgeable: ShockwaveDodgeable,
    pub duration: Duration,
    pub owner: Option<Uid>,
    pub specifier: FrontendSpecifier,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Shockwave {
    pub properties: Properties,
    #[serde(skip)]
    /// Time that the shockwave was created at
    /// Used to calculate shockwave propagation
    /// Deserialized from the network as `None`
    pub creation: Option<f64>,
}

impl Component for Shockwave {
    type Storage = DerefFlaggedStorage<Self, specs::DenseVecStorage<Self>>;
}

impl std::ops::Deref for Shockwave {
    type Target = Properties;

    fn deref(&self) -> &Properties { &self.properties }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ShockwaveHitEntities {
    pub hit_entities: Vec<Uid>,
}

impl Component for ShockwaveHitEntities {
    type Storage = specs::DenseVecStorage<Self>;
}

#[derive(Copy, Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum FrontendSpecifier {
    Ground,
    Fire,
    Water,
    Ice,
    IceSpikes,
    Steam,
    Poison,
    Ink,
    Lightning,
}

impl ShockwaveDodgeable {
    pub fn to_attack_source(&self) -> AttackSource {
        match self {
            Self::Roll => AttackSource::AirShockwave,
            Self::Jump => AttackSource::GroundShockwave,
            Self::No => AttackSource::UndodgeableShockwave,
        }
    }
}
