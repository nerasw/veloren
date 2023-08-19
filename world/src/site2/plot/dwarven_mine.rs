/*  Credits:
Zesterer: Code, Testing
Isse: Code
Gemu: Sprite-Art/NPC-Design and 3D Models
Floppy: NPC-Integration, Combat-Skill, Code and Animations
Sam: NPC Combat-Skills
Pfau: Level Design, NPC Concepts, Story, Code
Dialex: Testing, Site-Icon
Froggy: Testing
Imbris: Honorable Mention
ChatGPT: Improving Code Quality, Speeding up finding Syntax Errors
*/

use super::*;
use crate::{site2::gen::PrimitiveTransform, Land};
use common::{generation::SpecialEntity, terrain::Structure as PrefabStructure};
use rand::prelude::*;
use vek::*;

const TILE_SIZE: i32 = 13;

/// Represents house data generated by the `generate()` method
pub struct DwarvenMine {
    name: String,
    /// Axis aligned bounding region for the house
    bounds: Aabr<i32>,
    /// Approximate altitude of the door tile
    pub(crate) alt: i32,
    origin: Vec2<i32>,
}

impl DwarvenMine {
    pub fn generate(
        land: &Land,
        rng: &mut impl Rng,
        site: &Site,
        wpos: Vec2<i32>,
        tile_aabr: Aabr<i32>,
    ) -> Self {
        let bounds = Aabr {
            min: site.tile_wpos(tile_aabr.min),
            max: site.tile_wpos(tile_aabr.max),
        };

        let name = format!("{} Mine", NameGen::location(rng).generate_mine());

        // TODO: i18n for the word "mine"

        Self {
            name,
            bounds,
            origin: wpos - TILE_SIZE / 2,
            alt: land.get_alt_approx(site.tile_center_wpos(wpos)) as i32,
        }
    }

    pub fn name(&self) -> &str { &self.name }

    pub fn spawn_rules(&self, wpos: Vec2<i32>) -> SpawnRules {
        let near_entrance = wpos.distance_squared(self.origin) < 64i32.pow(2);
        SpawnRules {
            trees: !near_entrance,
            max_warp: (!near_entrance) as i32 as f32,
            ..SpawnRules::default()
        }
    }
}

impl Structure for DwarvenMine {
    #[cfg(feature = "use-dyn-lib")]
    const UPDATE_FN: &'static [u8] = b"render_mine\0";

    #[cfg_attr(feature = "be-dyn-lib", export_name = "render_mine")]

    fn render_inner(&self, _site: &Site, _land: &Land, painter: &Painter) {
        let center = self.bounds.center();
        // Entrance
        let entrance_path = "site_structures.dwarves.entrance";
        let entrance_pos: Vec3<i32> = (center.x, center.y, self.alt - 37).into(); //+2
        render_prefab(entrance_path, entrance_pos, painter);
        // Hallway 0
        let hallway0_path = "site_structures.dwarves.hallway";
        let hallway0_pos: Vec3<i32> = entrance_pos + Vec3::new(-67, 8, -37);
        render_prefab(hallway0_path, hallway0_pos + Vec3::new(0, 0, 0), painter);
        // Hallway 1
        let hallway1_path = "site_structures.dwarves.hallway1";
        let hallway1_pos: Vec3<i32> = hallway0_pos + Vec3::new(-55, -7, -33);
        render_prefab(hallway1_path, hallway1_pos, painter);
        // Mining
        let mining_site_path = "site_structures.dwarves.mining_site";
        let mining_site_pos: Vec3<i32> = hallway1_pos + Vec3::new(-5, 28, 21);
        render_prefab(mining_site_path, mining_site_pos, painter);
        // After Mining
        let after_mining_path = "site_structures.dwarves.after_flamekeeper";
        let after_mining_pos: Vec3<i32> = mining_site_pos + Vec3::new(-12, -9, -3);
        render_prefab(
            after_mining_path,
            after_mining_pos + Vec3::new(0, -2, 0),
            painter,
        );
        // Hallway 2
        let hallway2_path = "site_structures.dwarves.hallway2";
        let hallway2_pos: Vec3<i32> = after_mining_pos + Vec3::new(-3, 35, -18);
        render_prefab(hallway2_path, hallway2_pos, painter);
        // Excavation Site
        let excavation_site_path = "site_structures.dwarves.excavation_site";
        let excavation_site_pos: Vec3<i32> = hallway2_pos + Vec3::new(-35, 83, -10);
        render_prefab(excavation_site_path, excavation_site_pos, painter);
        // Flamekeeper Boss
        let flamekeeper_boss_path = "site_structures.dwarves.flamekeeper_boss";
        let flamekeeper_boss_pos: Vec3<i32> = excavation_site_pos + Vec3::new(115, 0, -17);
        render_prefab(flamekeeper_boss_path, flamekeeper_boss_pos, painter);
        // Cleansing Room
        let cleansing_room_path = "site_structures.dwarves.cleansing_room";
        let cleansing_room_pos: Vec3<i32> = excavation_site_pos + Vec3::new(-47, -125, 6);
        render_prefab(cleansing_room_path, cleansing_room_pos, painter);
        // Smelting Room
        let smelting_room_path = "site_structures.dwarves.smelting_room";
        let smelting_room_pos: Vec3<i32> = cleansing_room_pos + Vec3::new(49, -6, 0);
        render_prefab(smelting_room_path, smelting_room_pos, painter);
        // Flamekeeper Room
        let flamekeeper_path = "site_structures.dwarves.flamekeeper_room";
        let flamekeeper_pos: Vec3<i32> = smelting_room_pos + Vec3::new(8, 36, 10);
        render_prefab(flamekeeper_path, flamekeeper_pos, painter);

        // Spawn random NPCs/Rotated Sprites
        let random_entities: Vec<(Vec3<i32>, u8)> = vec![
            // Excavation Room
            (
                excavation_site_pos + Vec3::new(1, 42, 20) + Vec3::new(-4, -86, -10),
                4,
            ),
            // Near Dwarven Defense
            (excavation_site_pos + Vec3::new(-37, 35, 10), 0),
            // Smelting Room under stairs
            (smelting_room_pos + Vec3::new(-9, 11, 3), 0),
            (smelting_room_pos + Vec3::new(19, 11, 3), 0),
            // Treasure Rooms near Captain
            (flamekeeper_pos + Vec3::new(-16, 12, 1), 0),
            (flamekeeper_pos + Vec3::new(-16, 19, 1), 0),
            // Hallway back to the top
            (hallway1_pos + Vec3::new(-11, -5, 1), 4),
        ];

        for (pos, rot) in random_entities {
            spawn_random_entity(pos, painter, rot);
        }

        //Entities
        let miner = "common.entity.dungeon.dwarven_quarry.miner";
        let clockwork = "common.entity.dungeon.dwarven_quarry.clockwork";
        let turret = "common.entity.dungeon.dwarven_quarry.turret";
        let flamethrower = "common.entity.dungeon.dwarven_quarry.flamethrower";
        let mine_guard = "common.entity.dungeon.dwarven_quarry.mine_guard";
        let overseer = "common.entity.dungeon.dwarven_quarry.overseer";
        let flamekeeper = "common.entity.dungeon.dwarven_quarry.flamekeeper";
        let captain = "common.entity.dungeon.dwarven_quarry.captain";
        let minotaur = "common.entity.dungeon.dwarven_quarry.minotaur";
        let hoplite = "common.entity.dungeon.dwarven_quarry.hoplite";
        let marksman = "common.entity.dungeon.dwarven_quarry.marksman";
        let sniper = "common.entity.dungeon.dwarven_quarry.sniper";
        let strategian = "common.entity.dungeon.dwarven_quarry.strategian";
        let cyclops = "common.entity.dungeon.dwarven_quarry.cyclops";
        let alligator = "common.entity.dungeon.dwarven_quarry.alligator";

        let entrance_offset: Vec3<f32> = (20.0, -20.0, 45.0).into();
        // Spawn waypoint
        let waypoint_pos = (entrance_pos + Vec3::new(1, -1, 4)).map(|x| x as f32) + entrance_offset;
        painter
            .spawn(EntityInfo::at(waypoint_pos.map(|e| e)).into_special(SpecialEntity::Waypoint));

        let miner_pos: Vec<(Vec3<f32>, &str)> = vec![
            // Entrance
            (
                (entrance_pos + Vec3::new(-6, 0, 0)).map(|x| x as f32) + entrance_offset,
                miner,
            ),
            (
                (entrance_pos + Vec3::new(-2, 0, 0)).map(|x| x as f32) + entrance_offset,
                miner,
            ),
            // Hallway0
            (
                hallway0_pos.map(|x| x as f32) + Vec3::new(10.0, -4.0, 16.0),
                miner,
            ),
            (
                hallway0_pos.map(|x| x as f32) + Vec3::new(10.0, -6.0, 16.0),
                miner,
            ),
            (
                hallway0_pos.map(|x| x as f32) + Vec3::new(-10.0, -4.0, 10.0),
                miner,
            ),
            (
                hallway0_pos.map(|x| x as f32) + Vec3::new(-10.0, -6.0, 10.0),
                miner,
            ),
            (
                hallway0_pos.map(|x| x as f32) + Vec3::new(-10.0, -4.0, 10.0),
                miner,
            ),
            (
                hallway0_pos.map(|x| x as f32) + Vec3::new(-10.0, -6.0, 10.0),
                miner,
            ),
            // Hallway1
            (
                hallway0_pos.map(|x| x as f32) + Vec3::new(-30.0, -4.0, 0.0),
                miner,
            ),
            (
                hallway0_pos.map(|x| x as f32) + Vec3::new(-30.0, -6.0, 0.0),
                miner,
            ),
            (
                hallway0_pos.map(|x| x as f32) + Vec3::new(-46.0, -4.0, -5.0),
                miner,
            ),
            (
                hallway0_pos.map(|x| x as f32) + Vec3::new(-46.0, -6.0, -5.0),
                miner,
            ),
            // Mining
            (
                mining_site_pos.map(|x| x as f32) + Vec3::new(12.0, -5.0, 0.0),
                miner,
            ),
            (
                mining_site_pos.map(|x| x as f32) + Vec3::new(16.0, -5.0, 0.0),
                miner,
            ),
            (
                mining_site_pos.map(|x| x as f32) + Vec3::new(28.0, -5.0, 0.0),
                miner,
            ),
            (
                mining_site_pos.map(|x| x as f32) + Vec3::new(28.0, 0.0, 0.0),
                miner,
            ),
            (
                mining_site_pos.map(|x| x as f32) + Vec3::new(32.0, 0.0, 0.0),
                clockwork,
            ),
            (
                mining_site_pos.map(|x| x as f32) + Vec3::new(34.0, 32.0, 0.0),
                turret,
            ),
            (
                mining_site_pos.map(|x| x as f32) + Vec3::new(28.0, 6.0, 10.0),
                mine_guard,
            ),
            (
                mining_site_pos.map(|x| x as f32) + Vec3::new(28.0, 10.0, 10.0),
                mine_guard,
            ),
            (
                mining_site_pos.map(|x| x as f32) + Vec3::new(-8.0, 32.0, 0.0),
                turret,
            ),
            (
                mining_site_pos.map(|x| x as f32) + Vec3::new(27.0, 21.0, 10.0),
                mine_guard,
            ),
            (
                mining_site_pos.map(|x| x as f32) + Vec3::new(27.0, 25.0, 10.0),
                mine_guard,
            ),
            (
                mining_site_pos.map(|x| x as f32) + Vec3::new(-8.0, 22.0, 0.0),
                miner,
            ),
            (
                mining_site_pos.map(|x| x as f32) + Vec3::new(-4.0, 22.0, 0.0),
                miner,
            ),
            (
                mining_site_pos.map(|x| x as f32) + Vec3::new(-4.0, 18.0, 0.0),
                clockwork,
            ),
            (
                mining_site_pos.map(|x| x as f32) + Vec3::new(-8.0, 5.0, 0.0),
                miner,
            ),
            (
                mining_site_pos.map(|x| x as f32) + Vec3::new(-4.0, 5.0, 0.0),
                miner,
            ),
            (
                mining_site_pos.map(|x| x as f32) + Vec3::new(-4.0, 2.0, 0.0),
                mine_guard,
            ),
            (
                mining_site_pos.map(|x| x as f32) + Vec3::new(11.0, 18.0, 0.0),
                overseer,
            ),
            (
                mining_site_pos.map(|x| x as f32) + Vec3::new(10.0, 14.0, 0.0),
                mine_guard,
            ),
            (
                mining_site_pos.map(|x| x as f32) + Vec3::new(12.0, 14.0, 0.0),
                mine_guard,
            ),
            // After Mining

            // Hallway 2
            (
                hallway2_pos.map(|x| x as f32) + Vec3::new(-4.0, -22.0, 20.0),
                miner,
            ),
            (
                hallway2_pos.map(|x| x as f32) + Vec3::new(-6.0, -22.0, 20.0),
                miner,
            ),
            (
                hallway2_pos.map(|x| x as f32) + Vec3::new(-4.0, 7.0, 10.0),
                miner,
            ),
            (
                hallway2_pos.map(|x| x as f32) + Vec3::new(-6.0, 7.0, 10.0),
                miner,
            ),
            (
                hallway2_pos.map(|x| x as f32) + Vec3::new(-5.0, 30.0, 0.0),
                mine_guard,
            ),
            (
                hallway2_pos.map(|x| x as f32) + Vec3::new(-3.0, 30.0, 0.0),
                mine_guard,
            ),
            // Excavation Site
            (
                hallway2_pos.map(|x| x as f32) + Vec3::new(0.0, 44.0, 0.0),
                clockwork,
            ),
            (
                hallway2_pos.map(|x| x as f32) + Vec3::new(2.0, 44.0, 0.0),
                clockwork,
            ),
            (
                hallway2_pos.map(|x| x as f32) + Vec3::new(0.0, 40.0, 0.0),
                clockwork,
            ),
            (
                hallway2_pos.map(|x| x as f32) + Vec3::new(2.0, 40.0, 0.0),
                clockwork,
            ),
            (
                flamekeeper_boss_pos.map(|x| x as f32) + Vec3::new(23.0, 3.0, 20.0),
                flamekeeper,
            ),
            (
                excavation_site_pos.map(|x| x as f32) + Vec3::new(2.0, -10.0, 18.0),
                flamethrower,
            ),
            (
                excavation_site_pos.map(|x| x as f32) + Vec3::new(2.0, -16.0, 18.0),
                flamethrower,
            ),
            (
                excavation_site_pos.map(|x| x as f32) + Vec3::new(2.0, -12.0, 18.0),
                mine_guard,
            ),
            (
                excavation_site_pos.map(|x| x as f32) + Vec3::new(2.0, -14.0, 18.0),
                mine_guard,
            ),
            // Guards before keeper
            // First Group
            (
                excavation_site_pos.map(|x| x as f32) + Vec3::new(30.0, 10.0, 10.0),
                clockwork,
            ),
            (
                excavation_site_pos.map(|x| x as f32) + Vec3::new(30.0, 15.0, 10.0),
                clockwork,
            ),
            (
                excavation_site_pos.map(|x| x as f32) + Vec3::new(32.0, 10.0, 10.0),
                clockwork,
            ),
            (
                excavation_site_pos.map(|x| x as f32) + Vec3::new(32.0, 15.0, 10.0),
                clockwork,
            ),
            // Group before boss
            (
                excavation_site_pos.map(|x| x as f32) + Vec3::new(40.0, 20.0, 10.0),
                clockwork,
            ),
            (
                excavation_site_pos.map(|x| x as f32) + Vec3::new(40.0, 25.0, 10.0),
                clockwork,
            ),
            (
                excavation_site_pos.map(|x| x as f32) + Vec3::new(42.0, 20.0, 10.0),
                clockwork,
            ),
            (
                excavation_site_pos.map(|x| x as f32) + Vec3::new(42.0, 25.0, 10.0),
                clockwork,
            ),
            // Stairs
            (
                excavation_site_pos.map(|x| x as f32) + Vec3::new(1.0, 42.0, 20.0),
                mine_guard,
            ),
            (
                excavation_site_pos.map(|x| x as f32) + Vec3::new(-1.0, 42.0, 20.0),
                mine_guard,
            ),
            // Minotaur
            (
                excavation_site_pos.map(|x| x as f32) + Vec3::new(-30.0, -48.0, 10.0),
                minotaur,
            ),
            // Cleansing Room
            // Cyclops
            (
                cleansing_room_pos.map(|x| x as f32) + Vec3::new(16.0, 45.0, 5.0),
                cyclops,
            ),
            // Alligator
            (
                cleansing_room_pos.map(|x| x as f32) + Vec3::new(-14.0, 60.0, -10.0),
                alligator,
            ),
            // Smelting Room
            (
                smelting_room_pos.map(|x| x as f32) + Vec3::new(-5.0, -10.0, 5.0),
                flamethrower,
            ),
            (
                smelting_room_pos.map(|x| x as f32) + Vec3::new(15.0, -10.0, 5.0),
                flamethrower,
            ),
            // Treasure Room
            (
                flamekeeper_pos.map(|x| x as f32) + Vec3::new(11.0, 20.0, 2.0),
                flamethrower,
            ),
            (
                flamekeeper_pos.map(|x| x as f32) + Vec3::new(11.0, 20.0, 8.0),
                captain,
            ),
        ];

        // Entity Groups
        let entity_group_positions = vec![
            // Entrance
            (
                entrance_pos.map(|x| x as f32) + Vec3::new(2.0, -16.0, 1.0),
                vec![miner],
                2,
                5.0,
                3.0,
            ),
            // An
            // Excavation Site
            (
                excavation_site_pos.map(|x| x as f32) + Vec3::new(4.0, 20.0, 10.0),
                vec![miner, clockwork, mine_guard],
                4,
                5.0,
                3.0,
            ),
            // Ancient Corridor
            (
                excavation_site_pos.map(|x| x as f32) + Vec3::new(-20.0, -35.0, 10.0),
                vec![hoplite, marksman, strategian],
                4,
                5.0,
                4.0,
            ),
            (
                excavation_site_pos.map(|x| x as f32) + Vec3::new(-35.0, -35.0, 10.0),
                vec![hoplite, marksman, strategian],
                4,
                5.0,
                4.0,
            ),
            //
            (
                excavation_site_pos.map(|x| x as f32) + Vec3::new(-20.0, -20.0, 10.0),
                vec![hoplite, marksman, strategian],
                4,
                5.0,
                4.0,
            ),
            (
                excavation_site_pos.map(|x| x as f32) + Vec3::new(-35.0, -20.0, 10.0),
                vec![hoplite, marksman, strategian],
                4,
                5.0,
                4.0,
            ),
            //
            (
                excavation_site_pos.map(|x| x as f32) + Vec3::new(-20.0, -5.0, 10.0),
                vec![hoplite, marksman, strategian],
                4,
                5.0,
                4.0,
            ),
            (
                excavation_site_pos.map(|x| x as f32) + Vec3::new(-35.0, -5.0, 10.0),
                vec![hoplite, marksman, strategian],
                4,
                5.0,
                4.0,
            ),
            //Dwarven Defense
            (
                excavation_site_pos.map(|x| x as f32) + Vec3::new(-28.0, 15.0, 10.0),
                vec![miner, clockwork, mine_guard],
                8,
                5.0,
                4.0,
            ),
            // Cleansing Room First
            (
                cleansing_room_pos.map(|x| x as f32) + Vec3::new(12.0, 60.0, 5.0),
                vec![hoplite, marksman, strategian],
                4,
                5.0,
                4.0,
            ),
            (
                cleansing_room_pos.map(|x| x as f32) + Vec3::new(27.0, 60.0, 5.0),
                vec![hoplite, marksman, strategian],
                4,
                5.0,
                4.0,
            ),
            // Cleansing Room Second
            (
                cleansing_room_pos.map(|x| x as f32) + Vec3::new(20.0, 18.0, 5.0),
                vec![sniper],
                2,
                5.0,
                4.0,
            ),
            (
                cleansing_room_pos.map(|x| x as f32) + Vec3::new(-2.0, 18.0, 5.0),
                vec![sniper],
                2,
                5.0,
                4.0,
            ),
            (
                cleansing_room_pos.map(|x| x as f32) + Vec3::new(-2.0, -3.0, 5.0),
                vec![sniper],
                2,
                5.0,
                4.0,
            ),
            (
                cleansing_room_pos.map(|x| x as f32) + Vec3::new(19.0, -3.0, 5.0),
                vec![sniper],
                2,
                5.0,
                4.0,
            ),
            // Smelting Room
            (
                smelting_room_pos.map(|x| x as f32) + Vec3::new(-5.0, -10.0, 5.0),
                vec![clockwork],
                4,
                5.0,
                4.0,
            ),
            (
                smelting_room_pos.map(|x| x as f32) + Vec3::new(20.0, -10.0, 5.0),
                vec![clockwork],
                4,
                5.0,
                4.0,
            ),
            // Treasure Room
            (
                flamekeeper_pos.map(|x| x as f32) + Vec3::new(10.0, 10.0, 3.0),
                vec![miner, clockwork, mine_guard],
                4,
                5.0,
                4.0,
            ),
            // Staircase Back up
            (
                hallway1_pos.map(|x| x as f32) + Vec3::new(-5.0, 2.0, 0.0),
                vec![miner, clockwork, mine_guard],
                4,
                5.0,
                4.0,
            ),
            (
                hallway1_pos.map(|x| x as f32) + Vec3::new(-20.0, 10.0, 0.0),
                vec![miner, clockwork, mine_guard],
                4,
                5.0,
                4.0,
            ),
            (
                hallway1_pos.map(|x| x as f32) + Vec3::new(4.0, 15.0, 0.0),
                vec![miner, clockwork, mine_guard],
                4,
                5.0,
                4.0,
            ),
        ];

        for (position, entity_paths, num_entities, max_distance, entity_distance) in
            entity_group_positions
        {
            spawn_entities(
                position,
                painter,
                &entity_paths,
                num_entities,
                max_distance,
                entity_distance,
            );
        }

        /*
           ^-x
        < -y +y>

         */

        for (pos, entity) in miner_pos {
            spawn_entity(pos, painter, entity);
        }
    }
}
fn spawn_entity(pos: Vec3<f32>, painter: &Painter, entity_path: &str) {
    let mut rng = thread_rng();
    painter.spawn(
        EntityInfo::at(pos)
            .with_asset_expect(entity_path, &mut rng)
            .with_no_flee(),
    );
}

fn spawn_entities(
    pos: Vec3<f32>,
    painter: &Painter,
    entity_paths: &[&str],
    num_entities: u32,
    max_distance: f32,
    entity_distance: f32,
) {
    let mut rng = thread_rng();
    let num_paths = entity_paths.len();

    let side_length = (num_entities as f32).sqrt().ceil() as u32;
    let spacing = entity_distance;

    for i in 0..num_entities {
        let path_index = rng.gen_range(0..num_paths);
        let entity_path = entity_paths[path_index];

        let row = i / side_length;
        let col = i % side_length;

        let x_offset = col as f32 * spacing - max_distance;
        let y_offset = row as f32 * spacing - max_distance;

        let spawn_pos = pos + Vec3::new(x_offset, y_offset, 0.0);

        painter.spawn(EntityInfo::at(spawn_pos).with_asset_expect(entity_path, &mut rng));
    }
}

fn render_prefab(file_path: &str, position: Vec3<i32>, painter: &Painter) {
    let asset_handle = PrefabStructure::load_group(file_path);
    let prefab_structure = asset_handle.read()[0].clone();

    // Render the prefab
    painter
        .prim(Primitive::Prefab(Box::new(prefab_structure.clone())))
        .translate(position)
        .fill(Fill::Prefab(Box::new(prefab_structure), position, 0));
}

fn spawn_random_entity(pos: Vec3<i32>, painter: &Painter, rot: u8) {
    let mut rng = thread_rng();

    let entities = [
        "common.entity.dungeon.dwarven_quarry.miner",
        "common.entity.dungeon.dwarven_quarry.turret",
        "common.entity.dungeon.dwarven_quarry.clockwork",
    ];

    let sprites = [
        SpriteKind::FireBowlGround,
        SpriteKind::Bones,
        SpriteKind::Anvil,
        SpriteKind::Gold,
        SpriteKind::Forge,
        SpriteKind::RepairBench,
        SpriteKind::CommonLockedChest,
        SpriteKind::DungeonChest4,
        SpriteKind::DungeonChest5,
    ];

    let random_number = rng.gen_range(0..=10);

    if random_number <= 3 {
        let pos_f32 = pos.map(|coord| coord as f32);
        let random_entity_index = rng.gen_range(0..entities.len());
        let random_entity = entities[random_entity_index];
        spawn_entity(pos_f32, painter, random_entity);
    } else if random_number <= 9 {
        let random_sprite_index = rng.gen_range(0..sprites.len());
        let random_sprite = sprites[random_sprite_index];
        painter.rotated_sprite(pos, random_sprite, rot);
    } else {
        return;
    }
}
