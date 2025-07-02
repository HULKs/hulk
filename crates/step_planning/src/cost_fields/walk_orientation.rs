use geometry::look_at::LookAt;
use linear_algebra::Vector2;
use types::motion_command::OrientationMode;

use crate::{
    geometry::{angle::Angle, pose::Pose, pose::PoseGradient},
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
                // TODO(rmburg) scale importance by distance to target point
                if (point - pose.position).norm_squared() < 1e-5 {
                    0.0
                } else {
                    let orientation = pose.position.look_at(&point);

                    angle_penalty(pose.orientation, Angle(orientation.angle()))
                }
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
                if (point - pose.position).norm_squared() < 1e-5 {
                    PoseGradient {
                        position: Vector2::zeros(),
                        orientation: 0.0,
                    }
                } else {
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
}

#[cfg(test)]
mod tests {
    use std::f32::consts::TAU;

    use linear_algebra::{point, Orientation2};
    use proptest::proptest;
    use types::motion_command::OrientationMode;

    use crate::geometry::{angle::Angle, pose::Pose};

    use super::WalkOrientationField;

    proptest!(
        #[test]
        fn verify_gradient_look_towards(x in -5.0f32..5.0, y in -5.0f32..5.0, orientation in 0.0..TAU, target_orientation in 0.0..TAU) {
            let cost_field = WalkOrientationField {
                orientation_mode: OrientationMode::LookTowards(Orientation2::new(target_orientation)),
            };

            let position = point![x, y];
            let orientation = Angle(orientation);

            let pose = Pose {
                position,
                orientation,
            };

            crate::test_utils::verify_gradient::verify_gradient(
                &|p| cost_field.cost(p),
                &|p| cost_field.grad(p),
                0.05,
                pose,
            )
        }
    );

    proptest!(
        #[test]
        fn verify_gradient_look_at(x in -5.0f32..5.0, y in -5.0f32..5.0, orientation in 0.0..TAU, target_x in -5.0f32..5.0, target_y in -5.0f32..5.0) {
            let cost_field = WalkOrientationField {
                orientation_mode: OrientationMode::LookAt(point![target_x, target_y]),
            };

            let position = point![x, y];
            let orientation = Angle(orientation);

            // This only verifies the orientation field, since the
            // dependence on the position field is intentionally ignored
            crate::test_utils::verify_gradient::verify_gradient(
                &|orientation| {
                    let pose = Pose {
                        position,
                        orientation,
                    };

                    cost_field.cost(pose)
                },
                &|orientation| {
                    let pose = Pose {
                        position,
                        orientation,
                    };

                    cost_field.grad(pose).orientation
                },
                0.05,
                orientation,
            )
        }
    );
}
