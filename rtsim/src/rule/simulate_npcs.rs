use common::{terrain::TerrainChunkSize, vol::RectVolSize};
use tracing::info;
use vek::*;
use crate::{
    data::npc::NpcMode,
    event::OnTick,
    RtState, Rule, RuleError,
};

pub struct SimulateNpcs;

impl Rule for SimulateNpcs {
    fn start(rtstate: &mut RtState) -> Result<Self, RuleError> {

        rtstate.bind::<Self, OnTick>(|ctx| {
            let data = &mut *ctx.state.data_mut();
            for npc in data
                .npcs
                .values_mut()
                .filter(|npc| matches!(npc.mode, NpcMode::Simulated))
            {
                let body = npc.get_body();

                // Move NPCs if they have a target
                if let Some((target, speed_factor)) = npc.target {
                    npc.wpos += Vec3::from(
                        (target.xy() - npc.wpos.xy())
                            .try_normalized()
                            .unwrap_or_else(Vec2::zero)
                            * body.max_speed_approx()
                            * speed_factor,
                    ) * ctx.event.dt;
                }

                // Make sure NPCs remain on the surface
                npc.wpos.z = ctx.world.sim()
                    .get_alt_approx(npc.wpos.xy().map(|e| e as i32))
                    .unwrap_or(0.0);
                npc.current_site = ctx.world.sim().get(npc.wpos.xy().as_::<i32>() / TerrainChunkSize::RECT_SIZE.as_()).and_then(|chunk| {
                    data.sites.world_site_map.get(chunk.sites.first()?).copied()
                });
            }
        });

        Ok(Self)
    }
}
