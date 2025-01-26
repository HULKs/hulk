use geometry::look_at::LookAt;
use linear_algebra::Point2;
use types::motion_command::OrientationMode;

use crate::{
    geometry::{angle::Angle, Pose},
    traits::LossField,
    utils::{angle_penalty, angle_penalty_derivative},
};

pub struct WalkOrientationField {
    pub orientation_mode: OrientationMode,
}

impl LossField for WalkOrientationField {
    type Parameter = Pose<f32>;
    type Gradient = Pose<f32>;
    type Loss = f32;

    fn loss(&self, pose: Self::Parameter) -> Self::Loss {
        match self.orientation_mode {
            OrientationMode::AlignWithPath => 0.0,
            OrientationMode::LookTowards(orientation) => {
                angle_penalty(Angle(pose.orientation), Angle(orientation.angle()))
            }
            OrientationMode::LookAt(point) => {
                let orientation = pose.position.look_at(&point);

                angle_penalty(Angle(pose.orientation), Angle(orientation.angle()))
            }
        }
    }

    fn grad(&self, pose: Self::Parameter) -> Self::Gradient {
        match self.orientation_mode {
            OrientationMode::AlignWithPath => Pose {
                position: Point2::origin(),
                orientation: 0.0,
            },
            OrientationMode::LookTowards(orientation) => Pose {
                position: Point2::origin(),
                orientation: angle_penalty_derivative(
                    Angle(pose.orientation),
                    Angle(orientation.angle()),
                )
                .into_inner(),
            },
            OrientationMode::LookAt(point) => {
                let orientation = pose.position.look_at(&point);

                Pose {
                    position: Point2::origin(),
                    orientation: angle_penalty_derivative(
                        Angle(pose.orientation),
                        Angle(orientation.angle()),
                    )
                    .into_inner(),
                }
            }
        }
    }
}
