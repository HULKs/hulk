use coordinate_systems::Ground;
use geometry::look_at::LookAt;
use linear_algebra::{Orientation2, Vector2};
use types::motion_command::OrientationMode;

use crate::{
    geometry::{
        angle::Angle,
        pose::{Pose, PoseGradient},
    },
    utils::{
        angle_penalty, angle_penalty_derivative, angle_penalty_with_tolerance,
        angle_penalty_with_tolerance_derivative,
    },
};

pub struct WalkOrientationField {
    pub orientation_mode: OrientationMode,
    pub path_alignment_tolerance: f32,
}

impl WalkOrientationField {
    pub fn cost(&self, pose: Pose<f32>, forward: Vector2<Ground>) -> f32 {
        match self.orientation_mode {
            OrientationMode::Unspecified => 0.0,
            OrientationMode::AlignWithPath => angle_penalty_with_tolerance(
                pose.orientation,
                Angle(Orientation2::from_vector(forward).angle()),
                self.path_alignment_tolerance,
            ),
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

    pub fn grad(&self, pose: Pose<f32>, forward: Vector2<Ground>) -> PoseGradient<f32> {
        match self.orientation_mode {
            OrientationMode::Unspecified => PoseGradient::zeros(),
            OrientationMode::AlignWithPath => PoseGradient {
                orientation: angle_penalty_with_tolerance_derivative(
                    pose.orientation,
                    Angle(Orientation2::from_vector(forward).angle()),
                    self.path_alignment_tolerance,
                ),
                ..PoseGradient::zeros()
            },
            OrientationMode::LookTowards(orientation) => PoseGradient {
                orientation: angle_penalty_derivative(pose.orientation, Angle(orientation.angle())),
                ..PoseGradient::zeros()
            },
            OrientationMode::LookAt(point) => {
                if (point - pose.position).norm_squared() < 1e-5 {
                    PoseGradient::zeros()
                } else {
                    let orientation = pose.position.look_at(&point);

                    PoseGradient {
                        orientation: angle_penalty_derivative(
                            pose.orientation,
                            Angle(orientation.angle()),
                        ),
                        ..PoseGradient::zeros()
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::f32::consts::TAU;

    use linear_algebra::{point, Orientation2, Vector2};
    use proptest::proptest;
    use types::motion_command::OrientationMode;

    use crate::geometry::{angle::Angle, pose::Pose};

    use super::WalkOrientationField;

    proptest!(
        #[test]
        fn verify_gradient_look_towards(x in -5.0f32..5.0, y in -5.0f32..5.0, orientation in 0.0..TAU, target_orientation in 0.0..TAU) {
            let cost_field = WalkOrientationField {
                orientation_mode: OrientationMode::LookTowards(Orientation2::new(target_orientation)),
                path_alignment_tolerance: 1.0
            };

            let position = point![x, y];
            let orientation = Angle(orientation);

            let pose = Pose {
                position,
                orientation,
            };

            crate::test_utils::verify_gradient::verify_gradient(
                &|p| cost_field.cost(p, Vector2::x_axis()),
                &|p| cost_field.grad(p, Vector2::x_axis()),
                0.05,
                pose,
            )
        }
    );

    proptest!(
        #[test]
        fn verify_gradient_look_at(x in -5.0f32..5.0, y in -5.0f32..5.0, orientation in 0.0..TAU, target_x in -5.0f32..5.0, target_y in -5.0f32..5.0) {
            let cost_field = WalkOrientationField {
                orientation_mode: OrientationMode::LookAt(point![target_x,target_y]),
                path_alignment_tolerance: 1.0
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

                    cost_field.cost(pose, Vector2::x_axis())
                },
                &|orientation| {
                    let pose = Pose {
                        position,
                        orientation,
                    };

                    cost_field.grad(pose, Vector2::x_axis()).orientation
                },
                0.05,
                orientation,
            )
        }
    );

    proptest!(
        #[test]
        fn verify_gradient_align_with_path(x in -5.0f32..5.0, y in -5.0f32..5.0, orientation in 0.0..TAU, path_angle in 0.0..TAU) {
            let cost_field = WalkOrientationField {
                orientation_mode: OrientationMode::AlignWithPath,
                path_alignment_tolerance: 1.0
            };

            let position = point![x, y];
            let orientation = Angle(orientation);
            let forward_vector = Orientation2::new(path_angle).as_unit_vector();

            // This only verifies the orientation field, since the
            // dependence on the position field is intentionally ignored
            crate::test_utils::verify_gradient::verify_gradient(
                &|orientation| {
                    let pose = Pose {
                        position,
                        orientation,
                    };

                    cost_field.cost(pose, forward_vector)
                },
                &|orientation| {
                    let pose = Pose {
                        position,
                        orientation,
                    };

                    cost_field.grad(pose, forward_vector).orientation
                },
                0.05,
                orientation,
            )
        }
    );
}
