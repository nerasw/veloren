use std::f32::consts::PI;

use super::{
    super::{vek::*, Animation},
    ArthropodSkeleton, SkeletonAttr,
};
use common::states::utils::StageSection;

pub struct SummonAnimation;

impl Animation for SummonAnimation {
    type Dependency<'a> = (f32, f32, Option<StageSection>, f32);
    type Skeleton = ArthropodSkeleton;

    #[cfg(feature = "use-dyn-lib")]
    const UPDATE_FN: &'static [u8] = b"arthropod_summon\0";

    #[cfg_attr(feature = "be-dyn-lib", export_name = "arthropod_summon")]
    fn update_skeleton_inner(
        skeleton: &Self::Skeleton,
        (_velocity, global_time, stage_section, timer): Self::Dependency<'_>,
        anim_time: f32,
        _rate: &mut f32,
        s_a: &SkeletonAttr,
    ) -> Self::Skeleton {
        let mut next = (*skeleton).clone();

        let (movement1, movement2, movement3) = match stage_section {
            Some(StageSection::Buildup) => (anim_time.powi(2), 0.0, 0.0),
            Some(StageSection::Action) => (1.0, anim_time.powi(4), 0.0),
            Some(StageSection::Recover) => (1.0, 1.0, anim_time),
            _ => (0.0, 0.0, 0.0),
        };
        let pullback = 1.0 - movement3;
        let subtract = global_time - timer;
        let check = subtract - subtract.trunc();
        let mirror = (check - 0.5).signum();
        let movement1abs = movement1 * pullback;
        let movement2abs = movement2 * pullback;
        let movement3abs = movement3 * pullback;

        next.chest.scale = Vec3::one() * s_a.scaler;
        next.chest.orientation = Quaternion::rotation_x(movement2abs * 0.3)
            * Quaternion::rotation_z((movement1abs * 4.0 * PI).sin() * 0.02);

        next.head.position = Vec3::new(
            0.0,
            s_a.head.0 + movement1abs * 3.0,
            s_a.head.1 + movement1abs * -3.0,
        );
        next.head.orientation =
            Quaternion::rotation_x(movement1abs * 1.5 + movement2abs * -1.5 + movement3abs * 0.8)
                * Quaternion::rotation_y(
                    mirror * movement1abs * -0.2 + mirror * movement2abs * 0.2,
                )
                * Quaternion::rotation_z((movement1abs * 4.0 * PI).sin() * 0.02);

        next.chest.position = Vec3::new(
            0.0,
            s_a.chest.0,
            s_a.chest.1 + movement1abs * 7.0 + movement2abs * -2.0,
        );
        next.chest.orientation = Quaternion::rotation_x(movement1abs * -1.0 + movement2abs * 0.2);
        next.mandible_l.position = Vec3::new(-s_a.mandible.0, s_a.mandible.1, s_a.mandible.2);
        next.mandible_r.position = Vec3::new(s_a.mandible.0, s_a.mandible.1, s_a.mandible.2);
        next.mandible_l.orientation =
            Quaternion::rotation_x(movement1abs * 0.5 + movement2abs * -1.5 + movement3abs * 0.8)
                * Quaternion::rotation_z(
                    movement1abs * 0.5 + movement2abs * -0.6 + movement3abs * 0.8,
                );
        next.mandible_r.orientation =
            Quaternion::rotation_x(movement1abs * 0.5 + movement2abs * -1.5 + movement3abs * 0.8)
                * Quaternion::rotation_z(
                    movement1abs * -0.5 + movement2abs * 0.6 + movement3abs * -0.8,
                );

        next.leg_fl.position = Vec3::new(-s_a.leg_f.0, s_a.leg_f.1, s_a.leg_f.2);
        next.leg_fr.position = Vec3::new(s_a.leg_f.0, s_a.leg_f.1, s_a.leg_f.2);
        next.leg_fl.orientation = Quaternion::rotation_x(movement1abs * 1.0 + movement2abs * 0.2)
            * Quaternion::rotation_z(movement1abs * -0.2 + movement2abs * -0.2);
        next.leg_fr.orientation = Quaternion::rotation_x(movement1abs * 1.0 + movement2abs * 0.2)
            * Quaternion::rotation_x(movement1abs * 0.2 + movement2abs * 0.2);

        next.leg_fcl.position = Vec3::new(-s_a.leg_fc.0, s_a.leg_fc.1, s_a.leg_fc.2);
        next.leg_fcr.position = Vec3::new(s_a.leg_fc.0, s_a.leg_fc.1, s_a.leg_fc.2);

        next.leg_fcl.orientation = Quaternion::rotation_x(movement1abs * 1.3 + movement2abs * 0.3)
            * Quaternion::rotation_z(movement1abs * -0.5 + movement2abs * -0.2);
        next.leg_fcr.orientation = Quaternion::rotation_x(movement1abs * 1.3 + movement2abs * 0.3)
            * Quaternion::rotation_z(movement1abs * 0.5 + movement2abs * -0.2);

        next.leg_bcl.position = Vec3::new(-s_a.leg_bc.0, s_a.leg_bc.1, s_a.leg_bc.2);
        next.leg_bcr.position = Vec3::new(s_a.leg_bc.0, s_a.leg_bc.1, s_a.leg_bc.2);

        next.leg_bcl.orientation = Quaternion::rotation_x(movement1abs * 0.5 + movement2abs * 0.2);
        next.leg_bcr.orientation = Quaternion::rotation_x(movement1abs * 0.5 + movement2abs * 0.2);

        next.leg_bl.position = Vec3::new(-s_a.leg_b.0, s_a.leg_b.1, s_a.leg_b.2);
        next.leg_br.position = Vec3::new(s_a.leg_b.0, s_a.leg_b.1, s_a.leg_b.2);

        next.leg_bl.orientation = Quaternion::rotation_x(movement1abs * -0.5 + movement2abs * -0.2)
            * Quaternion::rotation_z(movement1abs * 0.8);
        next.leg_br.orientation = Quaternion::rotation_x(movement1abs * -0.5 + movement2abs * -0.2)
            * Quaternion::rotation_z(movement1abs * -0.8);
        next
    }
}
