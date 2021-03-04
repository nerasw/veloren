use crate::{
    persistence::character_updater,
    presence::Presence,
    sys::{SysScheduler, SysTimer},
};
use common::{
    comp::{Inventory, Stats, Waypoint},
    vsystem::{Origin, Phase, VJob, VSystem},
};
use common_net::msg::PresenceKind;
use specs::{Join, ReadExpect, ReadStorage, Write};

#[derive(Default)]
pub struct Sys;

impl<'a> VSystem<'a> for Sys {
    #[allow(clippy::type_complexity)] // TODO: Pending review in #587
    type SystemData = (
        ReadStorage<'a, Presence>,
        ReadStorage<'a, Stats>,
        ReadStorage<'a, Inventory>,
        ReadStorage<'a, Waypoint>,
        ReadExpect<'a, character_updater::CharacterUpdater>,
        Write<'a, SysScheduler<Self>>,
        Write<'a, SysTimer<Self>>,
    );

    const NAME: &'static str = "persistence";
    const ORIGIN: Origin = Origin::Server;
    const PHASE: Phase = Phase::Create;

    fn run(
        _job: &mut VJob<Self>,
        (
            presences,
            player_stats,
            player_inventories,
            player_waypoint,
            updater,
            mut scheduler,
            mut timer,
        ): Self::SystemData,
    ) {
        if scheduler.should_run() {
            timer.start();
            updater.batch_update(
                (
                    &presences,
                    &player_stats,
                    &player_inventories,
                    player_waypoint.maybe(),
                )
                    .join()
                    .filter_map(
                        |(presence, stats, inventory, waypoint)| match presence.kind {
                            PresenceKind::Character(id) => Some((id, stats, inventory, waypoint)),
                            PresenceKind::Spectator => None,
                        },
                    ),
            );
            timer.end();
        }
    }
}
