use super::*;
use crate::{
    util::{RandomField, Sampler, CARDINALS},
    Land,
};
use common::{
    generation::SpecialEntity,
    terrain::{BlockKind, SpriteKind},
};
use rand::prelude::*;
use std::sync::Arc;
use vek::*;

/// Represents house data generated by the `generate()` method
pub struct CoastalWorkshop {
    /// Tile position of the door tile
    pub door_tile: Vec2<i32>,
    /// Axis aligned bounding region for the house
    bounds: Aabr<i32>,
    /// Approximate altitude of the door tile
    pub(crate) alt: i32,
}

impl CoastalWorkshop {
    pub fn generate(
        land: &Land,
        _rng: &mut impl Rng,
        site: &Site,
        door_tile: Vec2<i32>,
        door_dir: Vec2<i32>,
        tile_aabr: Aabr<i32>,
    ) -> Self {
        let door_tile_pos = site.tile_center_wpos(door_tile);
        let bounds = Aabr {
            min: site.tile_wpos(tile_aabr.min),
            max: site.tile_wpos(tile_aabr.max),
        };
        Self {
            door_tile: door_tile_pos,
            bounds,
            alt: land.get_alt_approx(site.tile_center_wpos(door_tile + door_dir)) as i32 + 2,
        }
    }
}

impl Structure for CoastalWorkshop {
    #[cfg(feature = "use-dyn-lib")]
    const UPDATE_FN: &'static [u8] = b"render_coastalworkshop\0";

    #[cfg_attr(feature = "be-dyn-lib", export_name = "render_coastalworkshop")]
    fn render_inner(&self, _site: &Site, _land: &Land, painter: &Painter) {
        let base = self.alt + 1;
        let center = self.bounds.center();
        let white = Fill::Sampling(Arc::new(|center| {
            Some(match (RandomField::new(0).get(center)) % 37 {
                0..=8 => Block::new(BlockKind::Rock, Rgb::new(251, 251, 227)),
                9..=17 => Block::new(BlockKind::Rock, Rgb::new(245, 245, 229)),
                18..=26 => Block::new(BlockKind::Rock, Rgb::new(250, 243, 221)),
                27..=35 => Block::new(BlockKind::Rock, Rgb::new(240, 240, 230)),
                _ => Block::new(BlockKind::Rock, Rgb::new(255, 244, 193)),
            })
        }));
        let blue_broken = Fill::Sampling(Arc::new(|center| {
            Some(match (RandomField::new(0).get(center)) % 20 {
                0 => Block::new(BlockKind::Rock, Rgb::new(30, 187, 235)),
                _ => Block::new(BlockKind::Rock, Rgb::new(11, 146, 187)),
            })
        }));
        let length = (14 + RandomField::new(0).get(center.with_z(base)) % 3) as i32;
        let width = (12 + RandomField::new(0).get((center - 1).with_z(base)) % 3) as i32;
        let height = (12 + RandomField::new(0).get((center + 1).with_z(base)) % 4) as i32;

        // fence, blue gates
        painter
            .aabb(Aabb {
                min: Vec2::new(center.x - length - 6, center.y - width - 6).with_z(base - 2),
                max: Vec2::new(center.x + length + 7, center.y + width + 7).with_z(base - 1),
            })
            .fill(blue_broken.clone());

        for dir in CARDINALS {
            let frame_pos = Vec2::new(
                center.x + dir.x * (length + 5),
                center.y + dir.y * (width + 5),
            );
            painter
                .line(center.with_z(base - 1), frame_pos.with_z(base - 1), 3.0)
                .fill(blue_broken.clone());
        }
        // foundation
        painter
            .aabb(Aabb {
                min: Vec2::new(center.x - length - 6, center.y - width - 6).with_z(base - height),
                max: Vec2::new(center.x + length + 7, center.y + width + 7).with_z(base - 2),
            })
            .fill(white.clone());
        for f in 0..8 {
            painter
                .aabb(Aabb {
                    min: Vec2::new(center.x - length - 7 - f, center.y - width - 7 - f)
                        .with_z(base - 3 - f),
                    max: Vec2::new(center.x + length + 8 + f, center.y + width + 8 + f)
                        .with_z(base - 2 - f),
                })
                .fill(white.clone());
        }
        // clear yard
        painter
            .aabb(Aabb {
                min: Vec2::new(center.x - length - 5, center.y - width - 5).with_z(base - 2),
                max: Vec2::new(center.x + length + 6, center.y + width + 6).with_z(base + height),
            })
            .clear();
        // clear entries
        for dir in CARDINALS {
            let clear_pos = Vec2::new(
                center.x + dir.x * (length + 7),
                center.y + dir.y * (width + 7),
            );
            painter
                .line(center.with_z(base - 1), clear_pos.with_z(base - 1), 2.0)
                .clear();
        }
        // roof terrace
        painter
            .aabb(Aabb {
                min: Vec2::new(center.x - length - 3, center.y - width - 3)
                    .with_z(base - 3 + height),
                max: Vec2::new(center.x + length + 2, center.y + width + 2)
                    .with_z(base - 2 + height),
            })
            .fill(white.clone());
        painter
            .aabb(Aabb {
                min: Vec2::new(center.x - length - 3, center.y - width - 3)
                    .with_z(base - 2 + height),
                max: Vec2::new(center.x + length + 2, center.y + width + 2)
                    .with_z(base - 1 + height),
            })
            .fill(blue_broken.clone());
        painter
            .aabb(Aabb {
                min: Vec2::new(center.x - length - 2, center.y - width - 2)
                    .with_z(base - 2 + height),
                max: Vec2::new(center.x + length + 1, center.y + width + 1)
                    .with_z(base - 1 + height),
            })
            .clear();
        // room
        painter
            .aabb(Aabb {
                min: Vec2::new(center.x - length, center.y - width).with_z(base - 2),
                max: Vec2::new(center.x + length, center.y + width).with_z(base - 1),
            })
            .fill(blue_broken.clone());
        painter
            .aabb(Aabb {
                min: Vec2::new(center.x - length + 1, center.y - width + 1).with_z(base - 2),
                max: Vec2::new(center.x + length - 1, center.y + width - 1)
                    .with_z(base - 1 + height - 1),
            })
            .fill(white.clone());

        // entries
        let entry_limit = painter.aabb(Aabb {
            min: Vec2::new(center.x - length, center.y - width).with_z(base - 2),
            max: Vec2::new(center.x + length, center.y + width).with_z(base - 1 + height - 1),
        });
        painter
            .line(
                Vec2::new(center.x, center.y + 1 - width).with_z(base - 1),
                Vec2::new(center.x, center.y - 2 + width).with_z(base - 1),
                8.0,
            )
            .intersect(entry_limit)
            .fill(blue_broken.clone());
        painter
            .line(
                Vec2::new(center.x, center.y - width).with_z(base - 1),
                Vec2::new(center.x, center.y + width).with_z(base - 1),
                7.0,
            )
            .intersect(entry_limit)
            .clear();
        painter
            .line(
                Vec2::new(center.x + 1 - length, center.y).with_z(base - 1),
                Vec2::new(center.x - 2 + length, center.y).with_z(base - 1),
                8.0,
            )
            .intersect(entry_limit)
            .fill(blue_broken.clone());
        painter
            .line(
                Vec2::new(center.x - length, center.y).with_z(base - 1),
                Vec2::new(center.x + length, center.y).with_z(base - 1),
                7.0,
            )
            .intersect(entry_limit)
            .clear();
        // clear room
        painter
            .aabb(Aabb {
                min: Vec2::new(center.x - length + 2, center.y - width + 2).with_z(base - 2),
                max: Vec2::new(center.x + length - 2, center.y + width - 2)
                    .with_z(base - 2 + height - 1),
            })
            .clear();

        // room floors
        painter
            .aabb(Aabb {
                min: Vec2::new(center.x - length + 5, center.y - width + 5).with_z(base - 3),
                max: Vec2::new(center.x + length - 5, center.y + width - 5).with_z(base - 2),
            })
            .fill(blue_broken.clone());
        painter
            .aabb(Aabb {
                min: Vec2::new(center.x - length + 6, center.y - width + 6).with_z(base - 3),
                max: Vec2::new(center.x + length - 6, center.y + width - 6).with_z(base - 2),
            })
            .fill(white.clone());

        // wall lamps
        for d in 0..2 {
            let door_lamp_pos =
                Vec2::new(center.x - length + 2 + (d * ((2 * (length)) - 5)), center.y)
                    .with_z(base + 6);
            painter.rotated_sprite(
                door_lamp_pos,
                SpriteKind::WallLampSmall,
                2 + ((d * 4) as u8),
            );

            let lamp_pos = Vec2::new(center.x, center.y - width + 2 + (d * ((2 * (width)) - 5)))
                .with_z(base + 6);
            painter.rotated_sprite(lamp_pos, SpriteKind::WallLampSmall, 4 - ((d * 4) as u8));
        }
        for d in 0..2 {
            let door_lamp_pos =
                Vec2::new(center.x - length - 1 + (d * ((2 * (length)) + 1)), center.y)
                    .with_z(base + 6);
            painter.rotated_sprite(
                door_lamp_pos,
                SpriteKind::WallLampSmall,
                6 + ((d * 4) as u8),
            );

            let lamp_pos = Vec2::new(center.x, center.y - width - 1 + (d * ((2 * (width)) + 1)))
                .with_z(base + 6);
            painter.rotated_sprite(lamp_pos, SpriteKind::WallLampSmall, 8 - ((d * 4) as u8));
        }

        // chimney
        painter
            .cylinder(Aabb {
                min: (center - 4).with_z(base + height - 4),
                max: (center + 2).with_z(base - 2 + height + (height / 2)),
            })
            .fill(blue_broken);

        let top_limit = painter.aabb(Aabb {
            min: Vec2::new(center.x - length, center.y - width).with_z(base + height - 2),
            max: Vec2::new(center.x + length, center.y + width)
                .with_z(base - 2 + height + (height / 2)),
        });
        painter
            .superquadric(
                Aabb {
                    min: Vec2::new(center.x - length - 1, center.y - width - 1)
                        .with_z(base + height - (height / 2)),
                    max: Vec2::new(center.x + length, center.y + width)
                        .with_z(base - 2 + height + (height / 2)),
                },
                1.5,
            )
            .intersect(top_limit)
            .fill(white.clone());
        // clear chimney
        painter
            .cylinder(Aabb {
                min: (center - 3).with_z(base + height - 4),
                max: (center + 1).with_z(base - 2 + height + (height / 2)),
            })
            .clear();

        painter
            .cylinder(Aabb {
                min: (center - 3).with_z(base - 2),
                max: (center + 1).with_z(base - 1),
            })
            .fill(white);
        painter
            .aabb(Aabb {
                min: (center - 2).with_z(base - 2),
                max: (center).with_z(base - 1),
            })
            .clear();
        painter
            .aabb(Aabb {
                min: (center - 2).with_z(base - 3),
                max: (center).with_z(base - 2),
            })
            .fill(Fill::Block(Block::air(SpriteKind::Ember)));

        let mut stations = vec![
            SpriteKind::CraftingBench,
            SpriteKind::Forge,
            SpriteKind::SpinningWheel,
            SpriteKind::TanningRack,
            SpriteKind::CookingPot,
            SpriteKind::Cauldron,
            SpriteKind::Loom,
            SpriteKind::Anvil,
            SpriteKind::DismantlingBench,
            SpriteKind::RepairBench,
        ];
        'outer: for d in 0..3 {
            for dir in CARDINALS {
                if stations.is_empty() {
                    break 'outer;
                }
                let position = center + dir * (4 + d * 2);
                let cr_station = stations.swap_remove(
                    RandomField::new(0).get(position.with_z(base)) as usize % stations.len(),
                );
                painter.sprite(position.with_z(base - 2), cr_station);
            }
        }

        painter.spawn(
            EntityInfo::at((center - 2).with_z(base - 2).map(|e| e as f32 + 0.5))
                .into_special(SpecialEntity::Waypoint),
        );
    }
}
