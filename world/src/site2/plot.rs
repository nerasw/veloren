mod adlet;
mod airship_dock;
mod bridge;
mod castle;
mod citadel;
mod cliff_tower;
mod coastal_house;
mod coastal_workshop;
mod desert_city_arena;
mod desert_city_multiplot;
mod desert_city_temple;
pub mod dungeon;
mod dwarven_mine;
mod giant_tree;
mod gnarling;
mod house;
mod jungle_ruin;
mod pirate_hideout;
mod savannah_hut;
mod savannah_pit;
mod savannah_workshop;
mod sea_chapel;
mod workshop;

pub use self::{
    adlet::AdletStronghold, airship_dock::AirshipDock, bridge::Bridge, castle::Castle,
    citadel::Citadel, cliff_tower::CliffTower, coastal_house::CoastalHouse,
    coastal_workshop::CoastalWorkshop, desert_city_arena::DesertCityArena,
    desert_city_multiplot::DesertCityMultiPlot, desert_city_temple::DesertCityTemple,
    dungeon::Dungeon, dwarven_mine::DwarvenMine, giant_tree::GiantTree,
    gnarling::GnarlingFortification, house::House, jungle_ruin::JungleRuin,
    pirate_hideout::PirateHideout, savannah_hut::SavannahHut, savannah_pit::SavannahPit,
    savannah_workshop::SavannahWorkshop, sea_chapel::SeaChapel, workshop::Workshop,
};

use super::*;
use crate::util::DHashSet;
use common::path::Path;
use vek::*;

pub struct Plot {
    pub(crate) kind: PlotKind,
    pub(crate) root_tile: Vec2<i32>,
    pub(crate) tiles: DHashSet<Vec2<i32>>,
    pub(crate) seed: u32,
}

impl Plot {
    pub fn find_bounds(&self) -> Aabr<i32> {
        self.tiles
            .iter()
            .fold(Aabr::new_empty(self.root_tile), |b, t| {
                b.expanded_to_contain_point(*t)
            })
    }

    pub fn z_range(&self) -> Option<Range<i32>> {
        match &self.kind {
            PlotKind::House(house) => Some(house.z_range()),
            _ => None,
        }
    }

    pub fn kind(&self) -> &PlotKind { &self.kind }

    pub fn root_tile(&self) -> Vec2<i32> { self.root_tile }

    pub fn tiles(&self) -> impl ExactSizeIterator<Item = Vec2<i32>> + '_ {
        self.tiles.iter().copied()
    }
}

pub enum PlotKind {
    House(House),
    AirshipDock(AirshipDock),
    CoastalHouse(CoastalHouse),
    CoastalWorkshop(CoastalWorkshop),
    Workshop(Workshop),
    DesertCityMultiPlot(DesertCityMultiPlot),
    DesertCityTemple(DesertCityTemple),
    DesertCityArena(DesertCityArena),
    SeaChapel(SeaChapel),
    JungleRuin(JungleRuin),
    Plaza,
    Castle(Castle),
    Road(Path<Vec2<i32>>),
    Dungeon(Dungeon),
    Gnarling(GnarlingFortification),
    Adlet(AdletStronghold),
    GiantTree(GiantTree),
    CliffTower(CliffTower),
    Citadel(Citadel),
    SavannahPit(SavannahPit),
    SavannahHut(SavannahHut),
    SavannahWorkshop(SavannahWorkshop),
    Bridge(Bridge),
    PirateHideout(PirateHideout),
    //DwarvenMine(DwarvenMine),
}
