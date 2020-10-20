use super::{
    super::{vek::*, Animation},
    CharacterSkeleton, SkeletonAttr,
};
use common::{
    comp::item::{Hands, ToolKind},
    states::utils::StageSection,
};
use std::f32::consts::PI;
pub struct LeapAnimation;

impl Animation for LeapAnimation {
    type Dependency = (
        Option<ToolKind>,
        Option<ToolKind>,
        Vec3<f32>,
        f64,
        Option<StageSection>,
    );
    type Skeleton = CharacterSkeleton;

    #[cfg(feature = "use-dyn-lib")]
    const UPDATE_FN: &'static [u8] = b"character_leapmelee\0";

    #[cfg_attr(feature = "be-dyn-lib", export_name = "character_leapmelee")]
    #[allow(clippy::approx_constant)] // TODO: Pending review in #587
    fn update_skeleton_inner(
        skeleton: &Self::Skeleton,
        (active_tool_kind, second_tool_kind, _velocity, _global_time, stage_section): Self::Dependency,
        anim_time: f64,
        rate: &mut f32,
        skeleton_attr: &SkeletonAttr,
    ) -> Self::Skeleton {
        *rate = 1.0;
        let mut next = (*skeleton).clone();

        let (movement1, movement2, movement3, movement4) = match stage_section {
            Some(StageSection::Buildup) => (anim_time as f32, 0.0, 0.0, 0.0),
            Some(StageSection::Movement) => (1.0, anim_time as f32, 0.0, 0.0),
            Some(StageSection::Swing) => (1.0, 1.0, anim_time as f32, 0.0),
            Some(StageSection::Recover) => (1.0, 1.0, 1.0, anim_time as f32),
            _ => (0.0, 0.0, 0.0, 0.0),
        };

        if let Some(ToolKind::Hammer(_)) = active_tool_kind {
            next.hand_l.position = Vec3::new(-12.0, 0.0, 0.0);
            next.hand_l.orientation = Quaternion::rotation_x(PI) * Quaternion::rotation_y(0.0);
            next.hand_l.scale = Vec3::one() * 1.08;
            next.hand_r.position = Vec3::new(2.0, 0.0, 0.0);
            next.hand_r.orientation = Quaternion::rotation_x(PI) * Quaternion::rotation_y(0.0);
            next.hand_r.scale = Vec3::one() * 1.06;
            next.main.position = Vec3::new(0.0, 0.0, 0.0);
            next.main.orientation = Quaternion::rotation_y(-1.57) * Quaternion::rotation_z(1.57);

            next.head.position = Vec3::new(0.0, skeleton_attr.head.0, skeleton_attr.head.1);

            next.control.position = Vec3::new(
                6.0 + movement1 * -10.0,
                7.0 + movement2 * 5.0 + movement3 * 5.0,
                1.0 + movement2 * 5.0 + movement3 * -7.0,
            );
            next.control.orientation = Quaternion::rotation_x(0.3 + movement3 * -3.0)
                * Quaternion::rotation_y(0.0)
                * Quaternion::rotation_z(movement1 * 0.5 + movement2 * 0.5 + movement3 * 0.5);
            next.chest.orientation = Quaternion::rotation_x(
                movement1 * 0.3 + movement2 * 0.3 + movement3 * -0.9 + movement4 * 0.3,
            ) * Quaternion::rotation_y(0.0)
                * Quaternion::rotation_z(movement1 * 0.5 + movement2 * 0.2 + movement3 * -0.7);

            next.head.orientation = Quaternion::rotation_x(movement3 * 0.2)
                * Quaternion::rotation_y(0.0 + movement2 * -0.1)
                * Quaternion::rotation_z(movement1 * -0.4 + movement2 * -0.2 + movement3 * 0.6);

            next.hand_l.position = Vec3::new(-12.0 + movement3 * 10.0, 0.0, 0.0);

            next.foot_l.position = Vec3::new(
                -skeleton_attr.foot.0,
                skeleton_attr.foot.1 - 5.0 + movement3 * 13.0,
                skeleton_attr.foot.2 + movement3 * -5.0,
            );
            next.foot_l.orientation = Quaternion::rotation_x(-0.8 + movement3 * 1.7);

            next.foot_r.position = Vec3::new(
                skeleton_attr.foot.0,
                skeleton_attr.foot.1 + 8.0 + movement3 * -13.0,
                skeleton_attr.foot.2 + 5.0 + movement3 * -5.0,
            );
            next.foot_r.orientation = Quaternion::rotation_x(0.9 + movement3 * -1.7);
        } else if let Some(ToolKind::Axe(_)) = active_tool_kind {
            next.hand_l.position = Vec3::new(-0.5, 0.0, 4.0);
            next.hand_l.orientation = Quaternion::rotation_x(PI / 2.0)
                * Quaternion::rotation_z(0.0)
                * Quaternion::rotation_y(0.0);
            next.hand_r.position = Vec3::new(0.5, 0.0, -2.5);
            next.hand_r.orientation = Quaternion::rotation_x(PI / 2.0)
                * Quaternion::rotation_z(0.0)
                * Quaternion::rotation_y(0.0);
            next.main.position = Vec3::new(-0.0, -2.0, -1.0);
            next.main.orientation = Quaternion::rotation_x(0.0)
                * Quaternion::rotation_y(0.0)
                * Quaternion::rotation_z(0.0);

            next.control.position = Vec3::new(-3.0, 11.0, 3.0);
            next.control.orientation = Quaternion::rotation_x(1.8)
                * Quaternion::rotation_y(-0.5)
                * Quaternion::rotation_z(PI - 0.2);
            next.control.scale = Vec3::one();

            next.head.position = Vec3::new(0.0, skeleton_attr.head.0, skeleton_attr.head.1);

            next.control.position = Vec3::new(
                -3.0 + movement1 * 3.0,
                11.0 + movement1 * 1.0 + movement3 * 3.0,
                3.0 + movement1 * 12.0 + movement3 * -15.0,
            );
            next.control.orientation = Quaternion::rotation_x(
                1.8 + movement1 * -1.0 + movement2 * -0.5 + movement3 * -1.0,
            ) * Quaternion::rotation_y(-0.5 + movement1 * 0.5)
                * Quaternion::rotation_z(PI + 0.2 - movement1 * 0.2);

            next.torso.orientation = Quaternion::rotation_x(
                -0.3 + movement2 * -1.8 * PI + movement3 * -0.2 * PI + movement4 * -0.1 * PI,
            ) * Quaternion::rotation_y(0.0)
                * Quaternion::rotation_z(0.0);

            next.head.orientation =
                Quaternion::rotation_x(0.0 + movement1 * -0.4 + movement2 * 0.4 + movement3 * 0.2);

            next.foot_l.position = Vec3::new(
                skeleton_attr.foot.0,
                skeleton_attr.foot.1 + movement2 * 4.0 + movement3 * -8.0,
                skeleton_attr.foot.2 - 8.0 + movement2 * 3.0 + movement3 * -3.0,
            );

            next.foot_r.position = Vec3::new(
                skeleton_attr.foot.0,
                skeleton_attr.foot.1 + movement2 * 4.0 + movement3 * -8.0,
                skeleton_attr.foot.2 - 8.0 + movement2 * 3.0 + movement3 * -3.0,
            );

            next.foot_l.orientation = Quaternion::rotation_x(movement1 * 0.9 - movement3 * 1.8);

            next.foot_r.orientation = Quaternion::rotation_x(movement1 * 0.9 - movement3 * 1.8);

            next.belt.orientation = Quaternion::rotation_x(movement1 * 0.22 + movement2 * 0.1);
            next.shorts.orientation = Quaternion::rotation_x(movement1 * 0.3 + movement2 * 0.1);

            next.chest.position =
                Vec3::new(0.0, skeleton_attr.chest.0, skeleton_attr.chest.1 - 8.0);
            next.torso.position = Vec3::new(0.0, 0.0, 0.0 + 8.0) * skeleton_attr.scaler / 11.0;
        }

        next.second.scale = match (
            active_tool_kind.map(|tk| tk.hands()),
            second_tool_kind.map(|tk| tk.hands()),
        ) {
            (Some(Hands::OneHand), Some(Hands::OneHand)) => Vec3::one(),
            (_, _) => Vec3::zero(),
        };

        next
    }
}
