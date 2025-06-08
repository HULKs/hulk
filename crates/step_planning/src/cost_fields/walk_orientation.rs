use geometry::look_at::LookAt;
use linear_algebra::Vector2;
use types::motion_command::OrientationMode;

use crate::{
    geometry::{angle::Angle, pose::PoseGradient, Pose},
    utils::{angle_penalty, angle_penalty_derivative},
};

pub struct WalkOrientationField {
    pub orientation_mode: OrientationMode,
}

impl WalkOrientationField {
    pub fn cost(&self, pose: Pose<f32>) -> f32 {
        match self.orientation_mode {
            OrientationMode::Unspecified => 0.0,
            OrientationMode::LookTowards(orientation) => {
                angle_penalty(pose.orientation, Angle(orientation.angle()))
            }
            OrientationMode::LookAt(point) => {
                let orientation = pose.position.look_at(&point);

                angle_penalty(pose.orientation, Angle(orientation.angle()))
            }
        }
    }

    pub fn grad(&self, pose: Pose<f32>) -> PoseGradient<f32> {
        match self.orientation_mode {
            OrientationMode::Unspecified => PoseGradient {
                position: Vector2::zeros(),
                orientation: 0.0,
            },
            OrientationMode::LookTowards(orientation) => PoseGradient {
                position: Vector2::zeros(),
                orientation: angle_penalty_derivative(pose.orientation, Angle(orientation.angle())),
            },
            OrientationMode::LookAt(point) => {
                let orientation = pose.position.look_at(&point);

                PoseGradient {
                    position: Vector2::zeros(),
                    orientation: angle_penalty_derivative(
                        pose.orientation,
                        Angle(orientation.angle()),
                    ),
                }
            }
        }
    }
}
