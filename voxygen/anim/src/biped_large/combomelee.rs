use super::{
    super::{vek::*, Animation},
    BipedLargeSkeleton, SkeletonAttr,
};
use common::states::utils::{AbilityInfo, StageSection};
use core::f32::consts::PI;

pub struct ComboAnimation;
impl Animation for ComboAnimation {
    type Dependency<'a> = (
        Option<&'a str>,
        Option<StageSection>,
        Option<AbilityInfo>,
        usize,
        Vec2<f32>,
    );
    type Skeleton = BipedLargeSkeleton;

    #[cfg(feature = "use-dyn-lib")]
    const UPDATE_FN: &'static [u8] = b"biped_large_combo\0";

    #[cfg_attr(feature = "be-dyn-lib", export_name = "biped_large_combo")]
    fn update_skeleton_inner(
        skeleton: &Self::Skeleton,
        (ability_id, stage_section, _ability_info, current_strike, _move_dir): Self::Dependency<'_>,
        anim_time: f32,
        rate: &mut f32,
        s_a: &SkeletonAttr,
    ) -> Self::Skeleton {
        *rate = 1.0;
        let mut next = (*skeleton).clone();

        next.main.position = Vec3::new(0.0, 0.0, 0.0);
        next.main.orientation = Quaternion::rotation_z(0.0);
        next.second.position = Vec3::new(0.0, 0.0, 0.0);
        next.second.orientation = Quaternion::rotation_z(0.0);
        let multi_strike_pullback = 1.0
            - if matches!(stage_section, Some(StageSection::Recover)) {
                anim_time.powi(4)
            } else {
                0.0
            };

        for strike in 0..=current_strike {
            match ability_id {
                Some("common.abilities.adlet.elder.triplestrike") => {
                    let (move1, move2) = if strike == current_strike {
                        match stage_section {
                            Some(StageSection::Buildup) => {
                                (((anim_time.max(0.4) - 0.4) * 1.5).powf(0.5), 0.0)
                            },
                            Some(StageSection::Action) => (1.0, (anim_time.min(0.4) * 2.5).powi(2)),
                            Some(StageSection::Recover) => (1.0, 1.0),
                            _ => (0.0, 0.0),
                        }
                    } else {
                        (1.0, 1.0)
                    };
                    let move1 = move1 * multi_strike_pullback;
                    let move2 = move2 * multi_strike_pullback;
                    next.second.scale = Vec3::one() * 1.0;
                    next.main.position = Vec3::new(-s_a.grip.0 + 1.0, s_a.grip.0 * 2.0, 0.0);
                    next.second.position = Vec3::new(s_a.grip.0, s_a.grip.0 * 2.0, 0.0);
                    next.hand_l.position = Vec3::new(-s_a.grip.0, s_a.grip.0 + 1.0, 0.0);
                    next.hand_r.position = Vec3::new(s_a.grip.0, s_a.grip.0 + 1.0, 0.0);
                    next.hand_l.orientation = Quaternion::rotation_x(PI / 2.0);
                    next.hand_r.orientation = Quaternion::rotation_x(PI / 2.0);

                    next.main.orientation =
                        Quaternion::rotation_x(PI / -3.0) * Quaternion::rotation_z(PI / -4.0);
                    next.second.orientation =
                        Quaternion::rotation_x(PI / -3.0) * Quaternion::rotation_z(PI / 4.0);

                    next.head.orientation = Quaternion::rotation_x(move1 * -0.2 + move2 * 0.4);
                    match strike {
                        0 => {
                            next.weapon_l.position = Vec3::new(0.0, -8.0, -5.0);
                            next.weapon_l.orientation =
                                Quaternion::rotation_x(move1 * 0.3 + move2 * -0.2);
                            next.weapon_r.orientation =
                                Quaternion::rotation_z(move1 * -1.0 + move2 * 1.9);
                            next.shoulder_r.orientation = Quaternion::rotation_x(move1 * 1.0)
                                * Quaternion::rotation_y(move1 * -0.9 + move2 * 0.9);
                        },
                        1 => {
                            next.weapon_r.position = Vec3::new(0.0, -8.0, -5.0);
                            next.weapon_r.orientation =
                                Quaternion::rotation_x(move1 * 0.3 + move2 * -0.2);
                            next.weapon_l.orientation =
                                Quaternion::rotation_z(move1 * 1.0 + move2 * -1.9);
                            next.shoulder_l.orientation =
                                Quaternion::rotation_x(move1 * 1.0 * move2 * 1.0)
                                    * Quaternion::rotation_y(move1 * 0.9 + move2 * -0.9);
                        },

                        2 => {
                            next.weapon_l.orientation =
                                Quaternion::rotation_z(move1 * 1.0 + move2 * -1.2);
                            next.weapon_r.orientation =
                                Quaternion::rotation_z(move1 * -1.0 + move2 * 1.2);
                            next.shoulder_l.orientation =
                                Quaternion::rotation_x(move1 * 1.0 * move2 * 1.0)
                                    * Quaternion::rotation_y(move1 * 0.9 + move2 * -0.7);
                            next.shoulder_r.orientation =
                                Quaternion::rotation_x(move1 * 1.0 * move2 * 1.0)
                                    * Quaternion::rotation_y(move1 * -0.9 + move2 * 0.7);
                        },
                        _ => {},
                    }
                },
                _ => {},
            }
        }
        next
    }
}
