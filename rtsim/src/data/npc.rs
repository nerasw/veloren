use crate::ai::{Action, NpcCtx};
pub use common::rtsim::{NpcId, Profession};
use common::{
    comp,
    grid::Grid,
    rtsim::{FactionId, RtSimController, SiteId, VehicleId},
    store::Id,
    uid::Uid, vol::RectVolSize,
};
use hashbrown::HashMap;
use rand::prelude::*;
use serde::{Deserialize, Serialize};
use slotmap::HopSlotMap;
use std::{
    collections::VecDeque,
    ops::{Deref, DerefMut, Generator, GeneratorState},
    pin::Pin,
    sync::{
        atomic::{AtomicPtr, Ordering},
        Arc,
    },
};
use vek::*;
use world::{civ::Track, site::Site as WorldSite, util::{RandomPerm, LOCALITY}};

use super::Actor;

#[derive(Copy, Clone, Debug, Default)]
pub enum SimulationMode {
    /// The NPC is unloaded and is being simulated via rtsim.
    #[default]
    Simulated,
    /// The NPC has been loaded into the game world as an ECS entity.
    Loaded,
}

#[derive(Clone)]
pub struct PathData<P, N> {
    pub end: N,
    pub path: VecDeque<P>,
    pub repoll: bool,
}

#[derive(Clone, Default)]
pub struct PathingMemory {
    pub intrasite_path: Option<(PathData<Vec2<i32>, Vec2<i32>>, Id<WorldSite>)>,
    pub intersite_path: Option<(PathData<(Id<Track>, bool), SiteId>, usize)>,
}

#[derive(Clone, Copy)]
pub enum NpcAction {
    /// (wpos, speed_factor)
    Goto(Vec3<f32>, f32),
}

pub struct Controller {
    pub action: Option<NpcAction>,
}

impl Controller {
    pub fn idle() -> Self { Self { action: None } }

    pub fn goto(wpos: Vec3<f32>, speed_factor: f32) -> Self {
        Self {
            action: Some(NpcAction::Goto(wpos, speed_factor)),
        }
    }
}

pub struct Brain {
    pub action: Box<dyn Action<!>>,
}

#[derive(Serialize, Deserialize)]
pub struct Npc {
    // Persisted state
    /// Represents the location of the NPC.
    pub seed: u32,
    pub wpos: Vec3<f32>,

    pub profession: Option<Profession>,
    pub home: Option<SiteId>,
    pub faction: Option<FactionId>,

    pub riding: Option<Riding>,

    // Unpersisted state
    #[serde(skip_serializing, skip_deserializing)]
    pub chunk_pos: Option<Vec2<i32>>,
    #[serde(skip_serializing, skip_deserializing)]
    pub current_site: Option<SiteId>,

    #[serde(skip_serializing, skip_deserializing)]
    pub action: Option<NpcAction>,

    /// Whether the NPC is in simulated or loaded mode (when rtsim is run on the
    /// server, loaded corresponds to being within a loaded chunk). When in
    /// loaded mode, the interactions of the NPC should not be simulated but
    /// should instead be derived from the game.
    #[serde(skip_serializing, skip_deserializing)]
    pub mode: SimulationMode,

    #[serde(skip_serializing, skip_deserializing)]
    pub brain: Option<Brain>,
}

impl Clone for Npc {
    fn clone(&self) -> Self {
        Self {
            seed: self.seed,
            wpos: self.wpos,
            profession: self.profession.clone(),
            home: self.home,
            faction: self.faction,
            riding: self.riding.clone(),
            // Not persisted
            chunk_pos: None,
            current_site: Default::default(),
            action: Default::default(),
            mode: Default::default(),
            brain: Default::default(),
        }
    }
}

impl Npc {
    const PERM_BODY: u32 = 1;
    const PERM_SPECIES: u32 = 0;

    pub fn new(seed: u32, wpos: Vec3<f32>) -> Self {
        Self {
            seed,
            wpos,
            profession: None,
            home: None,
            faction: None,
            riding: None,
            chunk_pos: None,
            current_site: None,
            action: None,
            mode: SimulationMode::Simulated,
            brain: None,
        }
    }

    pub fn with_profession(mut self, profession: impl Into<Option<Profession>>) -> Self {
        self.profession = profession.into();
        self
    }

    pub fn with_home(mut self, home: impl Into<Option<SiteId>>) -> Self {
        self.home = home.into();
        self
    }

    pub fn steering(mut self, vehicle: impl Into<Option<VehicleId>>) -> Self {
        self.riding = vehicle.into().map(|vehicle| {
            Riding {
                vehicle,
                steering: true,
            }
        });
        self
    }

    pub fn riding(mut self, vehicle: impl Into<Option<VehicleId>>) -> Self {
        self.riding = vehicle.into().map(|vehicle| {
            Riding {
                vehicle,
                steering: false,
            }
        });
        self
    }

    pub fn with_faction(mut self, faction: impl Into<Option<FactionId>>) -> Self {
        self.faction = faction.into();
        self
    }

    pub fn rng(&self, perm: u32) -> impl Rng { RandomPerm::new(self.seed.wrapping_add(perm)) }

    pub fn get_body(&self) -> comp::Body {
        let species = *(&comp::humanoid::ALL_SPECIES)
            .choose(&mut self.rng(Self::PERM_SPECIES))
            .unwrap();
        comp::humanoid::Body::random_with(&mut self.rng(Self::PERM_BODY), &species).into()
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub struct Riding {
    pub vehicle: VehicleId,
    pub steering: bool,
}

#[derive(Clone, Serialize, Deserialize)]
pub enum VehicleKind {
    Airship,
    Boat,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct Vehicle {
    pub wpos: Vec3<f32>,

    pub kind: VehicleKind,

    #[serde(skip_serializing, skip_deserializing)]
    pub chunk_pos: Option<Vec2<i32>>,

    #[serde(skip_serializing, skip_deserializing)]
    pub driver: Option<Actor>,

    #[serde(skip_serializing, skip_deserializing)]
    // TODO: Find a way to detect riders when the vehicle is loaded
    pub riders: Vec<Actor>,

    /// Whether the Vehicle is in simulated or loaded mode (when rtsim is run on the
    /// server, loaded corresponds to being within a loaded chunk). When in
    /// loaded mode, the interactions of the Vehicle should not be simulated but
    /// should instead be derived from the game.
    #[serde(skip_serializing, skip_deserializing)]
    pub mode: SimulationMode,
}

impl Vehicle {
    pub fn new(wpos: Vec3<f32>, kind: VehicleKind) -> Self {
        Self {
            wpos,
            kind,
            chunk_pos: None,
            driver: None,
            riders: Vec::new(),
            mode: SimulationMode::Simulated,
        }
    }
    pub fn get_ship(&self) -> comp::ship::Body {
        match self.kind {
            VehicleKind::Airship => comp::ship::Body::DefaultAirship,
            VehicleKind::Boat => comp::ship::Body::Galleon,
        }
    }

    pub fn get_body(&self) -> comp::Body {
        comp::Body::Ship(self.get_ship())
    }

    /// Max speed in block/s
    pub fn get_speed(&self) -> f32 {
        match self.kind {
            VehicleKind::Airship => 15.0,
            VehicleKind::Boat => 13.0,
        }
    }
}

#[derive(Default, Clone, Serialize, Deserialize)]
pub struct GridCell {
    pub npcs: Vec<NpcId>,
    pub vehicles: Vec<VehicleId>,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct Npcs {
    pub npcs: HopSlotMap<NpcId, Npc>,
    pub vehicles: HopSlotMap<VehicleId, Vehicle>,
    #[serde(skip, default = "construct_npc_grid")]
    pub npc_grid: Grid<GridCell>,
}

fn construct_npc_grid() -> Grid<GridCell> { Grid::new(Vec2::zero(), Default::default()) }

impl Npcs {
    pub fn create_npc(&mut self, npc: Npc) -> NpcId { self.npcs.insert(npc) }

    pub fn create_vehicle(&mut self, vehicle: Vehicle) -> VehicleId { self.vehicles.insert(vehicle) }

    /// Queries nearby npcs, not garantueed to work if radius > 32.0
    pub fn nearby(&self, wpos: Vec2<f32>, radius: f32) -> impl Iterator<Item = NpcId> + '_ {
        let chunk_pos = wpos.as_::<i32>() / common::terrain::TerrainChunkSize::RECT_SIZE.as_::<i32>();
        let r_sqr = radius * radius;
        LOCALITY
            .into_iter()
            .filter_map(move |neighbor| {
                self
                    .npc_grid
                    .get(chunk_pos + neighbor)
                    .map(|cell| {
                        cell.npcs.iter()
                            .copied()
                            .filter(|npc| {
                                self.npcs.get(*npc)
                                    .map_or(false, |npc| npc.wpos.xy().distance_squared(wpos) < r_sqr)
                            })
                            .collect::<Vec<_>>()
                    })
            })
            .flatten()
    }
}

impl Deref for Npcs {
    type Target = HopSlotMap<NpcId, Npc>;

    fn deref(&self) -> &Self::Target { &self.npcs }
}

impl DerefMut for Npcs {
    fn deref_mut(&mut self) -> &mut Self::Target { &mut self.npcs }
}
