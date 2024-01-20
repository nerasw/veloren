use super::{
    super::{vek::*, Animation},
    biped_small_wield_sword, init_biped_small_alpha, BipedSmallSkeleton, SkeletonAttr,
};
use common::states::utils::StageSection;

pub struct RiposteMeleeAnimation;
impl Animation for RiposteMeleeAnimation {
    type Dependency<'a> = (Option<&'a str>, StageSection);
    type Skeleton = BipedSmallSkeleton;

    #[cfg(feature = "use-dyn-lib")]
    const UPDATE_FN: &'static [u8] = b"biped_small_riposte_melee\0";

    #[cfg_attr(feature = "be-dyn-lib", export_name = "biped_small_riposte_melee")]
    fn update_skeleton_inner(
        skeleton: &Self::Skeleton,
        (ability_id, stage_section): Self::Dependency<'_>,
        anim_time: f32,
        rate: &mut f32,
        s_a: &SkeletonAttr,
    ) -> Self::Skeleton {
        *rate = 1.0;
        let mut next = (*skeleton).clone();

        init_biped_small_alpha(&mut next, s_a);

        match ability_id {
            Some("common.abilities.haniwa.soldier.riposte") => {
                let slow = (anim_time * 2.0).sin();
                biped_small_wield_sword(&mut next, s_a, 0.0, slow);

                let (move1, move2, move3) = match stage_section {
                    StageSection::Buildup => (anim_time.powf(0.25), 0.0, 0.0),
                    StageSection::Action => (1.0, anim_time, 0.0),
                    StageSection::Recover => (1.0, 1.0, anim_time),
                    _ => (0.0, 0.0, 0.0),
                };
                let pullback = 1.0 - move3;
                let move1 = move1 * pullback;
                let move2 = move2 * pullback;
                let move2fast = move2.max(0.001).powf(0.25) * pullback;
                let move2slow = move2.powi(4) * pullback;

                next.detach_right = true;
                // For some reason there's a discontinuity when using detach_right, two offsets
                // below seem to help
                next.control_r.position += next.control.position
                    + Vec3::new(0.0, -2.0, 1.0)
                    + Vec3::new(2.0 * move3, -1.0 * move3, 2.0 * move3);
                next.control_r.orientation = next.control.orientation * next.control_r.orientation;

                next.control.orientation.rotate_z(move1 * 1.9);
                next.control.position += Vec3::new(0.0 * move1, 2.0 * move1, 13.0 * move1);
                next.control.orientation.rotate_y(move1 * 2.7);

                next.control.orientation.rotate_y(move2fast * -2.1);
                next.control.orientation.rotate_z(move2 * -1.8);
                next.control.orientation.rotate_x(move2slow * -3.2);
                next.control.position += Vec3::new(move2 * 3.0, move2 * -2.0, move2 * -10.0);
            },
            _ => {},
        }

        next
    }
}
