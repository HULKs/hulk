use coordinate_systems::Ground;
use linear_algebra::{Orientation2, Vector2};
use types::motion_command::OrientationMode;

use crate::{
    geometry::{
        angle::Angle,
        pose::{Pose, PoseGradient},
    },
    utils::{angle_penalty_with_tolerance, angle_penalty_with_tolerance_derivative},
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
            OrientationMode::LookTowards {
                direction,
                tolerance,
            } => {
                angle_penalty_with_tolerance(pose.orientation, Angle(direction.angle()), tolerance)
            }
            OrientationMode::LookAt { target, tolerance } => {
                let direction = target - pose.position;

                if direction.norm_squared() < 1e-5 {
                    0.0
                } else {
                    angle_penalty_with_tolerance(
                        pose.orientation,
                        Angle(Orientation2::from_vector(direction).angle()),
                        tolerance,
                    )
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
            OrientationMode::LookTowards {
                direction,
                tolerance,
            } => PoseGradient {
                orientation: angle_penalty_with_tolerance_derivative(
                    pose.orientation,
                    Angle(direction.angle()),
                    tolerance,
                ),
                ..PoseGradient::zeros()
            },
            OrientationMode::LookAt { target, tolerance } => {
                let direction = target - pose.position;

                if direction.norm_squared() < 1e-5 {
                    PoseGradient::zeros()
                } else {
                    PoseGradient {
                        orientation: angle_penalty_with_tolerance_derivative(
                            pose.orientation,
                            Angle(Orientation2::from_vector(direction).angle()),
                            tolerance,
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

    use coordinate_systems::Ground;
    use geometry::look_at::LookAt;
    use linear_algebra::{point, Orientation2, Point2, Vector2};
    use proptest::{prop_assume, proptest};
    use types::motion_command::OrientationMode;

    use crate::{
        geometry::{angle::Angle, pose::Pose},
        test_utils::{is_roughly_opposite, proptest_config},
    };

    use super::WalkOrientationField;

    proptest!(
        #![proptest_config(proptest_config())]
        #[test]
        fn verify_gradient_look_towards(x in -5.0f32..5.0, y in -5.0f32..5.0, orientation in 0.0..TAU, target_orientation in 0.0..TAU) {
            prop_assume!(!is_roughly_opposite(orientation, target_orientation));
            verify_gradient_look_towards_impl(x, y, orientation, target_orientation)
        }
    );

    fn verify_gradient_look_towards_impl(
        x: f32,
        y: f32,
        orientation: f32,
        target_orientation: f32,
    ) {
        let cost_field = WalkOrientationField {
            orientation_mode: OrientationMode::LookTowards {
                direction: Orientation2::new(target_orientation),
                tolerance: 0.0,
            },
            path_alignment_tolerance: 1.0,
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

    proptest!(
        #![proptest_config(proptest_config())]
        #[test]
        fn verify_gradient_look_at(x in -5.0f32..5.0, y in -5.0f32..5.0, orientation in 0.0..TAU, target_x in -5.0f32..5.0, target_y in -5.0f32..5.0) {
            let target: Point2<Ground> = point![target_x, target_y];
            prop_assume!(!is_roughly_opposite(orientation, point![x, y].look_at(&target).angle()));
            verify_gradient_look_at_impl(x, y, orientation, target_x, target_y)
        }
    );

    fn verify_gradient_look_at_impl(
        x: f32,
        y: f32,
        orientation: f32,
        target_x: f32,
        target_y: f32,
    ) {
        let cost_field = WalkOrientationField {
            orientation_mode: OrientationMode::LookAt {
                target: point![target_x, target_y],
                tolerance: 0.0,
            },
            path_alignment_tolerance: 1.0,
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

    proptest!(
        #![proptest_config(proptest_config())]
        #[test]
        fn verify_gradient_align_with_path(x in -5.0f32..5.0, y in -5.0f32..5.0, orientation in 0.0..TAU, path_angle in 0.0..TAU) {
            prop_assume!(!is_roughly_opposite(orientation, path_angle));
            verify_gradient_align_with_path_impl(x, y, orientation, path_angle)
        }
    );

    fn verify_gradient_align_with_path_impl(x: f32, y: f32, orientation: f32, path_angle: f32) {
        let cost_field = WalkOrientationField {
            orientation_mode: OrientationMode::AlignWithPath,
            path_alignment_tolerance: 1.0,
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
}
