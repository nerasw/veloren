pub mod feed;
pub mod idle;
pub mod jump;
pub mod run;

// Reexports
pub use self::{feed::FeedAnimation, idle::IdleAnimation, jump::JumpAnimation, run::RunAnimation};

use super::{make_bone, vek::*, FigureBoneData, Skeleton};
use common::comp::{self};
use core::convert::TryFrom;

pub type Body = comp::quadruped_small::Body;

skeleton_impls!(struct QuadrupedSmallSkeleton {
    + head,
    + chest,
    + leg_fl,
    + leg_fr,
    + leg_bl,
    + leg_br,
    + tail,
});

impl Skeleton for QuadrupedSmallSkeleton {
    type Attr = SkeletonAttr;
    type Body = Body;

    const BONE_COUNT: usize = 7;
    #[cfg(feature = "use-dyn-lib")]
    const COMPUTE_FN: &'static [u8] = b"quadruped_small_compute_mats\0";

    #[cfg_attr(feature = "be-dyn-lib", export_name = "quadruped_small_compute_mats")]
    fn compute_matrices_inner(
        &self,
        base_mat: Mat4<f32>,
        buf: &mut [FigureBoneData; super::MAX_BONE_COUNT],
    ) -> Vec3<f32> {
        let chest_mat = base_mat * Mat4::<f32>::from(self.chest);

        *(<&mut [_; Self::BONE_COUNT]>::try_from(&mut buf[0..Self::BONE_COUNT]).unwrap()) = [
            make_bone(chest_mat * Mat4::<f32>::from(self.head)),
            make_bone(chest_mat),
            make_bone(chest_mat * Mat4::<f32>::from(self.leg_fl)),
            make_bone(chest_mat * Mat4::<f32>::from(self.leg_fr)),
            make_bone(chest_mat * Mat4::<f32>::from(self.leg_bl)),
            make_bone(chest_mat * Mat4::<f32>::from(self.leg_br)),
            make_bone(chest_mat * Mat4::<f32>::from(self.tail)),
        ];
        Vec3::default()
    }
}

pub struct SkeletonAttr {
    head: (f32, f32),
    chest: (f32, f32),
    feet_f: (f32, f32, f32),
    feet_b: (f32, f32, f32),
    tail: (f32, f32),
    scaler: f32,
    tempo: f32,
    maximize: f32,
    minimize: f32,
    spring: f32,
    feed: f32,
}
impl<'a> std::convert::TryFrom<&'a comp::Body> for SkeletonAttr {
    type Error = ();

    fn try_from(body: &'a comp::Body) -> Result<Self, Self::Error> {
        match body {
            comp::Body::QuadrupedSmall(body) => Ok(SkeletonAttr::from(body)),
            _ => Err(()),
        }
    }
}

impl Default for SkeletonAttr {
    fn default() -> Self {
        Self {
            head: (0.0, 0.0),
            chest: (0.0, 0.0),
            feet_f: (0.0, 0.0, 0.0),
            feet_b: (0.0, 0.0, 0.0),
            tail: (0.0, 0.0),
            scaler: 0.0,
            tempo: 0.0,
            maximize: 0.0,
            minimize: 0.0,
            spring: 0.0,
            feed: 0.0,
        }
    }
}

impl<'a> From<&'a Body> for SkeletonAttr {
    fn from(body: &'a Body) -> Self {
        use comp::quadruped_small::Species::*;
        Self {
            head: match (body.species, body.body_type) {
                (Pig, _) => (5.0, 2.0),
                (Fox, _) => (4.0, 3.0),
                (Sheep, _) => (4.0, 4.0),
                (Boar, _) => (7.0, 0.0),
                (Jackalope, _) => (3.0, 2.0),
                (Skunk, _) => (5.0, 1.5),
                (Cat, _) => (4.0, 3.0),
                (Batfox, _) => (5.0, 1.0),
                (Raccoon, _) => (5.0, 2.0),
                (Quokka, _) => (6.0, 2.0),
                (Dodarock, _) => (6.0, -2.0),
                (Holladon, _) => (7.0, 1.0),
                (Hyena, _) => (7.5, 2.0),
                (Rabbit, _) => (4.0, 3.0),
                (Truffler, _) => (7.5, -9.0),
                (Frog, _) => (4.0, 2.0),
            },
            chest: match (body.species, body.body_type) {
                (Pig, _) => (0.0, 6.0),
                (Fox, _) => (0.0, 8.0),
                (Sheep, _) => (2.0, 7.0),
                (Boar, _) => (0.0, 9.5),
                (Jackalope, _) => (-2.0, 6.0),
                (Skunk, _) => (0.0, 6.0),
                (Cat, _) => (0.0, 6.0),
                (Batfox, _) => (-2.0, 6.0),
                (Raccoon, _) => (0.0, 5.5),
                (Quokka, _) => (2.0, 6.5),
                (Dodarock, _) => (-2.0, 9.0),
                (Holladon, _) => (-2.0, 9.0),
                (Hyena, _) => (-2.0, 9.0),
                (Rabbit, _) => (-2.0, 6.0),
                (Truffler, _) => (-2.0, 16.0),
                (Frog, _) => (-2.0, 4.5),
            },
            feet_f: match (body.species, body.body_type) {
                (Pig, _) => (4.5, 3.5, -1.0),
                (Fox, _) => (3.0, 5.0, -5.5),
                (Sheep, _) => (3.5, 2.0, -2.0),
                (Boar, _) => (3.5, 6.0, -5.5),
                (Jackalope, _) => (3.0, 4.0, -2.0),
                (Skunk, _) => (3.5, 4.0, -1.0),
                (Cat, _) => (2.0, 4.0, -1.0),
                (Batfox, _) => (3.0, 4.0, -0.5),
                (Raccoon, _) => (4.0, 4.0, -0.0),
                (Quokka, _) => (3.0, 4.0, -1.0),
                (Dodarock, _) => (5.0, 5.0, -2.5),
                (Holladon, _) => (5.0, 4.0, -2.5),
                (Hyena, _) => (2.5, 5.0, -4.0),
                (Rabbit, _) => (3.0, 3.0, -3.0),
                (Truffler, _) => (2.5, 5.0, -9.0),
                (Frog, _) => (4.5, 6.5, 0.0),
            },
            feet_b: match (body.species, body.body_type) {
                (Pig, _) => (3.5, -2.0, 0.0),
                (Fox, _) => (3.0, -3.0, -3.0),
                (Sheep, _) => (3.5, -3.5, -2.0),
                (Boar, _) => (3.0, -3.0, -2.5),
                (Jackalope, _) => (3.5, -2.0, 0.0),
                (Skunk, _) => (3.5, -4.0, -1.5),
                (Cat, _) => (2.0, -3.5, -1.0),
                (Batfox, _) => (3.5, -2.0, -0.5),
                (Raccoon, _) => (4.5, -3.0, 0.5),
                (Quokka, _) => (4.0, -4.0, -1.0),
                (Dodarock, _) => (3.5, -3.0, -4.0),
                (Holladon, _) => (4.0, -2.0, -3.0),
                (Hyena, _) => (3.0, -5.0, -2.5),
                (Rabbit, _) => (3.5, -2.0, -1.0),
                (Truffler, _) => (3.0, -5.0, -9.5),
                (Frog, _) => (5.0, -3.5, 0.0),
            },
            tail: match (body.species, body.body_type) {
                (Pig, _) => (-4.5, 2.5),
                (Fox, _) => (-4.5, 2.0),
                (Sheep, _) => (-5.0, 0.0),
                (Boar, _) => (-6.0, 0.0),
                (Jackalope, _) => (-4.0, 2.0),
                (Skunk, _) => (-4.0, 0.5),
                (Cat, _) => (-3.5, 2.0),
                (Batfox, _) => (0.0, 5.0),
                (Raccoon, _) => (-4.0, 1.0),
                (Quokka, _) => (-6.0, 1.0),
                (Dodarock, _) => (0.0, 5.0),
                (Holladon, _) => (-1.0, 4.0),
                (Hyena, _) => (-7.0, 0.0),
                (Rabbit, _) => (-4.0, -0.0),
                (Truffler, _) => (0.0, 0.0),
                (Frog, _) => (0.0, -0.0),
            },
            scaler: match (body.species, body.body_type) {
                (Pig, _) => (0.9),
                (Fox, _) => (0.9),
                (Sheep, _) => (1.0),
                (Boar, _) => (1.1),
                (Jackalope, _) => (0.8),
                (Skunk, _) => (0.9),
                (Cat, _) => (0.8),
                (Batfox, _) => (1.1),
                (Raccoon, _) => (1.0),
                (Quokka, _) => (1.0),
                (Dodarock, _) => (1.2),
                (Holladon, _) => (1.4),
                (Hyena, _) => (1.0),
                (Rabbit, _) => (0.7),
                (Truffler, _) => (1.0),
                (Frog, _) => (0.7),
            },
            tempo: match (body.species, body.body_type) {
                (Pig, _) => (1.0),
                (Fox, _) => (1.0),
                (Sheep, _) => (1.0),
                (Boar, _) => (1.1),
                (Jackalope, _) => (1.0),
                (Skunk, _) => (1.0),
                (Cat, _) => (1.1),
                (Batfox, _) => (1.0),
                (Raccoon, _) => (1.0),
                (Quokka, _) => (1.2),
                (Dodarock, _) => (1.0),
                (Holladon, _) => (1.0),
                (Hyena, _) => (1.1),
                (Rabbit, _) => (1.15),
                (Truffler, _) => (1.0),
                (Frog, _) => (1.15),
            },
            maximize: match (body.species, body.body_type) {
                (Pig, _) => (1.0),
                (Fox, _) => (1.3),
                (Sheep, _) => (1.1),
                (Boar, _) => (1.4),
                (Jackalope, _) => (1.2),
                (Skunk, _) => (1.0),
                (Cat, _) => (1.0),
                (Batfox, _) => (1.0),
                (Raccoon, _) => (1.0),
                (Quokka, _) => (1.0),
                (Dodarock, _) => (1.0),
                (Holladon, _) => (1.0),
                (Hyena, _) => (1.4),
                (Rabbit, _) => (1.3),
                (Truffler, _) => (1.0),
                (Frog, _) => (1.3),
            },
            minimize: match (body.species, body.body_type) {
                (Pig, _) => (0.6),
                (Fox, _) => (1.3),
                (Sheep, _) => (0.8),
                (Boar, _) => (1.0),
                (Jackalope, _) => (0.8),
                (Skunk, _) => (0.9),
                (Cat, _) => (0.8),
                (Batfox, _) => (1.0),
                (Raccoon, _) => (1.0),
                (Quokka, _) => (0.9),
                (Dodarock, _) => (0.9),
                (Holladon, _) => (0.7),
                (Hyena, _) => (1.4),
                (Rabbit, _) => (0.8),
                (Truffler, _) => (1.0),
                (Frog, _) => (0.8),
            },
            spring: match (body.species, body.body_type) {
                (Pig, _) => (1.0),
                (Fox, _) => (1.0),
                (Sheep, _) => (1.2),
                (Boar, _) => (0.8),
                (Jackalope, _) => (2.2),
                (Skunk, _) => (1.0),
                (Cat, _) => (1.4),
                (Batfox, _) => (1.1),
                (Raccoon, _) => (1.1),
                (Quokka, _) => (1.3),
                (Dodarock, _) => (0.9),
                (Holladon, _) => (0.7),
                (Hyena, _) => (1.4),
                (Rabbit, _) => (2.5),
                (Truffler, _) => (0.8),
                (Frog, _) => (2.5),
            },
            feed: match (body.species, body.body_type) {
                (Pig, _) => (1.0),
                (Fox, _) => (1.0),
                (Sheep, _) => (1.0),
                (Boar, _) => (0.6),
                (Jackalope, _) => (1.0),
                (Skunk, _) => (0.8),
                (Cat, _) => (1.0),
                (Batfox, _) => (0.7),
                (Raccoon, _) => (0.8),
                (Quokka, _) => (1.0),
                (Dodarock, _) => (0.7),
                (Holladon, _) => (1.0),
                (Hyena, _) => (1.0),
                (Rabbit, _) => (1.2),
                (Truffler, _) => (0.6),
                (Frog, _) => (0.7),
            },
        }
    }
}
