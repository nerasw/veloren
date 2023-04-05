pub mod faction;
pub mod name;
pub mod site;

use crate::data::{
    faction::{Faction, Factions},
    npc::{Npc, Npcs, Profession, Vehicle},
    site::{Site, Sites},
    Data, Nature,
};
use common::{
    comp::{self, Body},
    grid::Grid,
    resources::TimeOfDay,
    rtsim::{Personality, WorldSettings},
    terrain::TerrainChunkSize,
    vol::RectVolSize,
};
use rand::prelude::*;
use tracing::info;
use vek::*;
use world::{site::SiteKind, site2::PlotKind, IndexRef, World};

impl Data {
    pub fn generate(settings: &WorldSettings, world: &World, index: IndexRef) -> Self {
        let mut seed = [0; 32];
        seed.iter_mut()
            .zip(&mut index.seed.to_le_bytes())
            .for_each(|(dst, src)| *dst = *src);
        let mut rng = SmallRng::from_seed(seed);

        let mut this = Self {
            nature: Nature::generate(world),
            npcs: Npcs {
                npcs: Default::default(),
                vehicles: Default::default(),
                npc_grid: Grid::new(Vec2::zero(), Default::default()),
                character_map: Default::default(),
            },
            sites: Sites {
                sites: Default::default(),
                world_site_map: Default::default(),
            },
            factions: Factions {
                factions: Default::default(),
            },

            tick: 0,
            time_of_day: TimeOfDay(settings.start_time),
            should_purge: false,
        };

        let initial_factions = (0..16)
            .map(|_| {
                let faction = Faction::generate(world, index, &mut rng);
                let wpos = world
                    .sim()
                    .get_size()
                    .map2(TerrainChunkSize::RECT_SIZE, |e, sz| {
                        rng.gen_range(0..(e * sz) as i32)
                    });
                (wpos, this.factions.create(faction))
            })
            .collect::<Vec<_>>();
        info!("Generated {} rtsim factions.", this.factions.len());

        // Register sites with rtsim
        for (world_site_id, _) in index.sites.iter() {
            let site = Site::generate(
                world_site_id,
                world,
                index,
                &initial_factions,
                &this.factions,
            );
            this.sites.create(site);
        }
        info!(
            "Registering {} rtsim sites from world sites.",
            this.sites.len()
        );
        // Spawn some test entities at the sites
        for (site_id, site, site2) in this.sites.iter()
        // TODO: Stupid. Only find site2 towns
        .filter_map(|(site_id, site)| Some((site_id, site, site.world_site
            .and_then(|ws| match &index.sites.get(ws).kind {
                SiteKind::Refactor(site2)
                | SiteKind::CliffTown(site2)
                | SiteKind::SavannahPit(site2)
                | SiteKind::DesertCity(site2) => Some(site2),
                _ => None,
            })?)))
        {
            let Some(good_or_evil) = site
                .faction
                .and_then(|f| this.factions.get(f))
                .map(|f| f.good_or_evil)
            else { continue };

            let rand_wpos = |rng: &mut SmallRng, matches_plot: fn(&PlotKind) -> bool| {
                let wpos2d = site2
                    .plots()
                    .filter(|plot| matches_plot(plot.kind()))
                    .choose(&mut thread_rng())
                    .map(|plot| site2.tile_center_wpos(plot.root_tile()))
                    .unwrap_or_else(|| site.wpos.map(|e| e + rng.gen_range(-10..10)));
                wpos2d
                    .map(|e| e as f32 + 0.5)
                    .with_z(world.sim().get_alt_approx(wpos2d).unwrap_or(0.0))
            };
            let random_humanoid = |rng: &mut SmallRng| {
                let species = comp::humanoid::ALL_SPECIES.choose(&mut *rng).unwrap();
                Body::Humanoid(comp::humanoid::Body::random_with(rng, species))
            };
            let matches_buildings = (|kind: &PlotKind| {
                matches!(
                    kind,
                    PlotKind::House(_) | PlotKind::Workshop(_) | PlotKind::Plaza
                )
            }) as _;
            let matches_plazas = (|kind: &PlotKind| matches!(kind, PlotKind::Plaza)) as _;
            if good_or_evil {
                for _ in 0..site2.plots().len() {
                    this.npcs.create_npc(
                        Npc::new(
                            rng.gen(),
                            rand_wpos(&mut rng, matches_buildings),
                            random_humanoid(&mut rng),
                        )
                        .with_faction(site.faction)
                        .with_home(site_id)
                        .with_personality(Personality::random(&mut rng))
                        .with_profession(match rng.gen_range(0..20) {
                            0 => Profession::Hunter,
                            1 => Profession::Blacksmith,
                            2 => Profession::Chef,
                            3 => Profession::Alchemist,
                            5..=8 => Profession::Farmer,
                            9..=10 => Profession::Herbalist,
                            11..=16 => Profession::Guard,
                            _ => Profession::Adventurer(rng.gen_range(0..=3)),
                        }),
                    );
                }
            } else {
                for _ in 0..15 {
                    this.npcs.create_npc(
                        Npc::new(
                            rng.gen(),
                            rand_wpos(&mut rng, matches_buildings),
                            random_humanoid(&mut rng),
                        )
                        .with_personality(Personality::random_evil(&mut rng))
                        .with_faction(site.faction)
                        .with_home(site_id)
                        .with_profession(Profession::Cultist),
                    );
                }
            }
            // Merchants
            if good_or_evil {
                for _ in 0..(site2.plots().len() / 6) + 1 {
                    this.npcs.create_npc(
                        Npc::new(
                            rng.gen(),
                            rand_wpos(&mut rng, matches_plazas),
                            random_humanoid(&mut rng),
                        )
                        .with_home(site_id)
                        .with_personality(Personality::random_good(&mut rng))
                        .with_profession(Profession::Merchant),
                    );
                }
            }

            if rng.gen_bool(0.4) {
                let wpos = rand_wpos(&mut rng, matches_plazas) + Vec3::unit_z() * 50.0;
                let vehicle_id = this
                    .npcs
                    .create_vehicle(Vehicle::new(wpos, comp::body::ship::Body::DefaultAirship));

                this.npcs.create_npc(
                    Npc::new(rng.gen(), wpos, random_humanoid(&mut rng))
                        .with_home(site_id)
                        .with_profession(Profession::Captain)
                        .with_personality(Personality::random_good(&mut rng))
                        .steering(vehicle_id),
                );
            }
        }

        for (site_id, site) in this.sites.iter()
        // TODO: Stupid
        .filter(|(_, site)| site.world_site.map_or(false, |ws|
        matches!(&index.sites.get(ws).kind, SiteKind::Dungeon(_))))
        {
            let rand_wpos = |rng: &mut SmallRng| {
                let wpos2d = site.wpos.map(|e| e + rng.gen_range(-10..10));
                wpos2d
                    .map(|e| e as f32 + 0.5)
                    .with_z(world.sim().get_alt_approx(wpos2d).unwrap_or(0.0))
            };

            let species = [
                comp::body::bird_large::Species::Phoenix,
                comp::body::bird_large::Species::Cockatrice,
                comp::body::bird_large::Species::Roc,
            ]
            .choose(&mut rng)
            .unwrap();
            this.npcs.create_npc(
                Npc::new(
                    rng.gen(),
                    rand_wpos(&mut rng),
                    Body::BirdLarge(comp::body::bird_large::Body::random_with(&mut rng, species)),
                )
                .with_home(site_id),
            );
        }
        info!("Generated {} rtsim NPCs.", this.npcs.len());

        this
    }
}
