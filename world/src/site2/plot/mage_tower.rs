use super::*;
use crate::{
    assets::AssetHandle,
    site2::gen::PrimitiveTransform,
    util::{sampler::Sampler, RandomField},
    Land,
};
use common::{
    generation::EntityInfo,
    terrain::{Structure as PrefabStructure, StructuresGroup},
};
use lazy_static::lazy_static;
use rand::prelude::*;
use vek::*;

pub struct MageTower {
    bounds: Aabr<i32>,
    pub(crate) alt: i32,
}
impl MageTower {
    pub fn generate(land: &Land, _rng: &mut impl Rng, site: &Site, tile_aabr: Aabr<i32>) -> Self {
        let bounds = Aabr {
            min: site.tile_wpos(tile_aabr.min),
            max: site.tile_wpos(tile_aabr.max),
        };
        Self {
            bounds,
            alt: land.get_alt_approx(site.tile_center_wpos((tile_aabr.max - tile_aabr.min) / 2))
                as i32
                + 2,
        }
    }
}

impl Structure for MageTower {
    #[cfg(feature = "use-dyn-lib")]
    const UPDATE_FN: &'static [u8] = b"render_mage_tower\0";

    #[cfg_attr(feature = "be-dyn-lib", export_name = "render_mage_tower")]
    fn render_inner(&self, _site: &Site, land: &Land, painter: &Painter) {
        let center = self.bounds.center();
        let base = land.get_alt_approx(center) as i32;
        let mut thread_rng = thread_rng();
        let model_pos = center.with_z(base);
        // model
        lazy_static! {
            pub static ref MODEL: AssetHandle<StructuresGroup> =
                PrefabStructure::load_group("site_structures.mage_tower.mage_tower");
        }
        let rng = RandomField::new(0).get(model_pos) % 10;
        let model = MODEL.read();
        let model = model[rng as usize % model.len()].clone();
        painter
            .prim(Primitive::Prefab(Box::new(model.clone())))
            .translate(model_pos)
            .fill(Fill::Prefab(Box::new(model), model_pos, rng));
        // npcs
        // floor 0
        painter.spawn(
            EntityInfo::at(center.with_z(base + 2).as_()).with_asset_expect(
                "common.entity.spot.wizard.novice",
                &mut thread_rng,
                None,
            ),
        );
        // floor 1
        painter.spawn(
            EntityInfo::at(center.with_z(base + 19).as_()).with_asset_expect(
                "common.entity.spot.wizard.adept",
                &mut thread_rng,
                None,
            ),
        );
        // floor 2
        painter.spawn(
            EntityInfo::at(center.with_z(base + 37).as_()).with_asset_expect(
                "common.entity.spot.wizard.overseer",
                &mut thread_rng,
                None,
            ),
        );
        // floor 2 loot chanber
        let npc_amount = thread_rng.gen_range(0..=3);
        for _ in 0..npc_amount {
            let npc = match thread_rng.gen_range(0..=3) {
                0 => "common.entity.spot.wizard.novice",
                1 => "common.entity.spot.wizard.adept",
                2 => "common.entity.spot.wizard.overseer",
                _ => "common.entity.spot.wizard.spellbinder",
            };
            painter.spawn(
                EntityInfo::at((center - 15).with_z(base + 42).as_()).with_asset_expect(
                    npc,
                    &mut thread_rng,
                    None,
                ),
            );
        }
        // floor 4
        painter.spawn(
            EntityInfo::at(center.with_z(base + 84).as_()).with_asset_expect(
                "common.entity.spot.wizard.wizard_argo",
                &mut thread_rng,
                None,
            ),
        );
        // floor 5
        painter.spawn(
            EntityInfo::at(center.with_z(base + 98).as_()).with_asset_expect(
                "common.entity.spot.wizard.wizard_haku",
                &mut thread_rng,
                None,
            ),
        );
        // floor 6
        painter.spawn(
            EntityInfo::at(center.with_z(base + 111).as_()).with_asset_expect(
                "common.entity.spot.wizard.wizard_trish",
                &mut thread_rng,
                None,
            ),
        );
        painter.spawn(
            EntityInfo::at((center + 1).with_z(base + 111).as_()).with_asset_expect(
                "common.entity.spot.wizard.overseer",
                &mut thread_rng,
                None,
            ),
        );
        painter.spawn(
            EntityInfo::at((center - 1).with_z(base + 111).as_()).with_asset_expect(
                "common.entity.spot.wizard.spellbinder",
                &mut thread_rng,
                None,
            ),
        );
    }
}
