use super::*;
use common::{
    comp::Pos,
    event::{EventBus, ServerEvent},
    terrain::TerrainGrid,
    vsystem::{Origin, Phase, VJob, VSystem},
};
use specs::{Entities, Read, ReadExpect, ReadStorage, WriteExpect};

#[derive(Default)]
pub struct Sys;
impl<'a> VSystem<'a> for Sys {
    #[allow(clippy::type_complexity)]
    type SystemData = (
        Read<'a, EventBus<ServerEvent>>,
        WriteExpect<'a, RtSim>,
        ReadExpect<'a, TerrainGrid>,
        Entities<'a>,
        ReadStorage<'a, RtSimEntity>,
        ReadStorage<'a, Pos>,
    );

    const NAME: &'static str = "rtsim::unload_chunks";
    const ORIGIN: Origin = Origin::Server;
    const PHASE: Phase = Phase::Create;

    fn run(
        _job: &mut VJob<Self>,
        (
            _server_event_bus,
            mut rtsim,
            _terrain_grid,
            _entities,
            _rtsim_entities,
            _positions,
        ): Self::SystemData,
    ) {
        let chunks = std::mem::take(&mut rtsim.chunks.chunks_to_unload);

        for _chunk in chunks {
            // TODO
        }
    }
}
