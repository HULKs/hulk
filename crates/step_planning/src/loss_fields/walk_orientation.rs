use geometry::look_at::LookAt;
use linear_algebra::Point2;
use types::motion_command::OrientationMode;

use crate::{
    geometry::{angle::Angle, Pose},
    utils::{angle_penalty, angle_penalty_derivative},
};

pub struct WalkOrientationField {
    pub orientation_mode: OrientationMode,
}

impl WalkOrientationField {
    pub fn loss(&self, pose: Pose<f32>) -> f32 {
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

    pub fn grad(&self, pose: Pose<f32>) -> Pose<f32> {
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
