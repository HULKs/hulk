use std::f32::consts::PI;

use nalgebra::{matrix, point, Isometry2, Point2};
use serde::{Deserialize, Serialize};
use serialize_hierarchy::SerializeHierarchy;

#[derive(Clone, Copy, Debug, Default, Deserialize, Serialize, SerializeHierarchy)]
pub struct Interpolated {
    pub first_half_own_half_towards_own_goal: f32,
    pub first_half_own_half_away_own_goal: f32,
    pub first_half_opponent_half_towards_own_goal: f32,
    pub first_half_opponent_half_away_own_goal: f32,
}

impl Interpolated {
    const ARGUMENT_FIRST_HALF_OWN_HALF_TOWARDS_OWN_GOAL: Point2<f32> = point![-3.0, 0.0];
    const ARGUMENT_FIRST_HALF_OWN_HALF_AWAY_OWN_GOAL: Point2<f32> = point![-3.0, PI];
    const ARGUMENT_FIRST_HALF_OPPONENT_HALF_TOWARDS_OWN_GOAL: Point2<f32> = point![3.0, 0.0];
    const ARGUMENT_FIRST_HALF_OPPONENT_HALF_AWAY_OWN_GOAL: Point2<f32> = point![3.0, PI];

    pub fn evaluate_at(&self, robot_to_field: Isometry2<f32>) -> f32 {
        let argument = point![
            robot_to_field.translation.x,
            robot_to_field.rotation.angle().abs()
        ];

        assert_eq!(
            Self::ARGUMENT_FIRST_HALF_OWN_HALF_TOWARDS_OWN_GOAL.x,
            Self::ARGUMENT_FIRST_HALF_OWN_HALF_AWAY_OWN_GOAL.x,
        );
        assert_eq!(
            Self::ARGUMENT_FIRST_HALF_OPPONENT_HALF_TOWARDS_OWN_GOAL.x,
            Self::ARGUMENT_FIRST_HALF_OPPONENT_HALF_AWAY_OWN_GOAL.x,
        );
        assert_eq!(
            Self::ARGUMENT_FIRST_HALF_OWN_HALF_TOWARDS_OWN_GOAL.y,
            Self::ARGUMENT_FIRST_HALF_OPPONENT_HALF_TOWARDS_OWN_GOAL.y,
        );
        assert_eq!(
            Self::ARGUMENT_FIRST_HALF_OWN_HALF_AWAY_OWN_GOAL.y,
            Self::ARGUMENT_FIRST_HALF_OPPONENT_HALF_AWAY_OWN_GOAL.y,
        );

        let x1 = Self::ARGUMENT_FIRST_HALF_OWN_HALF_TOWARDS_OWN_GOAL.x;
        let x2 = Self::ARGUMENT_FIRST_HALF_OPPONENT_HALF_TOWARDS_OWN_GOAL.x;
        let y1 = Self::ARGUMENT_FIRST_HALF_OWN_HALF_TOWARDS_OWN_GOAL.y;
        let y2 = Self::ARGUMENT_FIRST_HALF_OWN_HALF_AWAY_OWN_GOAL.y;

        let factor = 1.0 / ((x2 - x1) * (y2 - y1));
        let evaluated_parameters = matrix![
            self.first_half_own_half_towards_own_goal,
            self.first_half_own_half_away_own_goal,
            self.first_half_opponent_half_towards_own_goal,
            self.first_half_opponent_half_away_own_goal
        ];
        let transformation = matrix![x2 * y2, -y2, -x2, 1.0;
                                     -x2 * y1, y1, x2, -1.0;
                                     -x1 * y2, y2, x1, -1.0;
                                     x1 * y1, -y1, -x1, 1.0];
        let argument = matrix![1.0; argument.x; argument.y; argument.x * argument.y];

        (factor * evaluated_parameters * transformation * argument).as_slice()[0]
    }
}

impl From<f32> for Interpolated {
    fn from(value: f32) -> Self {
        Self {
            first_half_own_half_towards_own_goal: value,
            first_half_own_half_away_own_goal: value,
            first_half_opponent_half_towards_own_goal: value,
            first_half_opponent_half_away_own_goal: value,
        }
    }
}

#[cfg(test)]
mod tests {
    use approx::assert_relative_eq;
    use nalgebra::{Rotation2, Translation2};

    use super::*;

    fn half_between(start: f32, end: f32) -> f32 {
        start + ((end - start) / 2.0)
    }

    #[test]
    fn arguments_result_in_parameters() {
        let interpolated = Interpolated {
            first_half_own_half_towards_own_goal: 0.0,
            first_half_own_half_away_own_goal: 1.0,
            first_half_opponent_half_towards_own_goal: 2.0,
            first_half_opponent_half_away_own_goal: 3.0,
        };

        let cases = [
            (
                Isometry2::from_parts(
                    Translation2::new(
                        Interpolated::ARGUMENT_FIRST_HALF_OWN_HALF_TOWARDS_OWN_GOAL.x,
                        0.0,
                    ),
                    Rotation2::new(Interpolated::ARGUMENT_FIRST_HALF_OWN_HALF_TOWARDS_OWN_GOAL.y)
                        .into(),
                ),
                0.0,
            ),
            (
                Isometry2::from_parts(
                    Translation2::new(
                        Interpolated::ARGUMENT_FIRST_HALF_OWN_HALF_AWAY_OWN_GOAL.x,
                        0.0,
                    ),
                    Rotation2::new(Interpolated::ARGUMENT_FIRST_HALF_OWN_HALF_AWAY_OWN_GOAL.y)
                        .into(),
                ),
                1.0,
            ),
            (
                Isometry2::from_parts(
                    Translation2::new(
                        Interpolated::ARGUMENT_FIRST_HALF_OPPONENT_HALF_TOWARDS_OWN_GOAL.x,
                        0.0,
                    ),
                    Rotation2::new(
                        Interpolated::ARGUMENT_FIRST_HALF_OPPONENT_HALF_TOWARDS_OWN_GOAL.y,
                    )
                    .into(),
                ),
                2.0,
            ),
            (
                Isometry2::from_parts(
                    Translation2::new(
                        Interpolated::ARGUMENT_FIRST_HALF_OPPONENT_HALF_AWAY_OWN_GOAL.x,
                        0.0,
                    ),
                    Rotation2::new(Interpolated::ARGUMENT_FIRST_HALF_OPPONENT_HALF_AWAY_OWN_GOAL.y)
                        .into(),
                ),
                3.0,
            ),
        ];

        for (robot_to_field, expected) in cases {
            dbg!((robot_to_field, expected));
            assert_relative_eq!(
                interpolated.evaluate_at(robot_to_field),
                expected,
                epsilon = 0.001
            );
        }
    }

    #[test]
    fn pairwise_center_arguments_result_in_pairwise_interpolated_values() {
        let interpolated = Interpolated {
            first_half_own_half_towards_own_goal: 0.0,
            first_half_own_half_away_own_goal: 1.0,
            first_half_opponent_half_towards_own_goal: 2.0,
            first_half_opponent_half_away_own_goal: 3.0,
        };

        let cases = [
            (
                Isometry2::from_parts(
                    Translation2::new(
                        half_between(
                            Interpolated::ARGUMENT_FIRST_HALF_OWN_HALF_TOWARDS_OWN_GOAL.x,
                            Interpolated::ARGUMENT_FIRST_HALF_OPPONENT_HALF_TOWARDS_OWN_GOAL.x,
                        ),
                        0.0,
                    ),
                    Rotation2::new(Interpolated::ARGUMENT_FIRST_HALF_OWN_HALF_TOWARDS_OWN_GOAL.y)
                        .into(),
                ),
                1.0,
            ),
            (
                Isometry2::from_parts(
                    Translation2::new(
                        Interpolated::ARGUMENT_FIRST_HALF_OPPONENT_HALF_TOWARDS_OWN_GOAL.x,
                        0.0,
                    ),
                    Rotation2::new(half_between(
                        Interpolated::ARGUMENT_FIRST_HALF_OPPONENT_HALF_TOWARDS_OWN_GOAL.y,
                        Interpolated::ARGUMENT_FIRST_HALF_OPPONENT_HALF_AWAY_OWN_GOAL.y,
                    ))
                    .into(),
                ),
                2.5,
            ),
            (
                Isometry2::from_parts(
                    Translation2::new(
                        half_between(
                            Interpolated::ARGUMENT_FIRST_HALF_OWN_HALF_AWAY_OWN_GOAL.x,
                            Interpolated::ARGUMENT_FIRST_HALF_OPPONENT_HALF_AWAY_OWN_GOAL.x,
                        ),
                        0.0,
                    ),
                    Rotation2::new(Interpolated::ARGUMENT_FIRST_HALF_OPPONENT_HALF_AWAY_OWN_GOAL.y)
                        .into(),
                ),
                2.0,
            ),
            (
                Isometry2::from_parts(
                    Translation2::new(
                        Interpolated::ARGUMENT_FIRST_HALF_OWN_HALF_TOWARDS_OWN_GOAL.x,
                        0.0,
                    ),
                    Rotation2::new(half_between(
                        Interpolated::ARGUMENT_FIRST_HALF_OPPONENT_HALF_TOWARDS_OWN_GOAL.y,
                        Interpolated::ARGUMENT_FIRST_HALF_OPPONENT_HALF_AWAY_OWN_GOAL.y,
                    ))
                    .into(),
                ),
                0.5,
            ),
        ];

        for (robot_to_field, expected) in cases {
            dbg!((robot_to_field, expected));
            assert_relative_eq!(
                interpolated.evaluate_at(robot_to_field),
                expected,
                epsilon = 0.001
            );
        }
    }

    #[test]
    fn center_argument_results_in_bilinear_interpolated_value() {
        let interpolated = Interpolated {
            first_half_own_half_towards_own_goal: 0.0,
            first_half_own_half_away_own_goal: 1.0,
            first_half_opponent_half_towards_own_goal: 2.0,
            first_half_opponent_half_away_own_goal: 3.0,
        };

        assert_relative_eq!(
            interpolated.evaluate_at(Isometry2::from_parts(
                Translation2::new(
                    half_between(
                        Interpolated::ARGUMENT_FIRST_HALF_OWN_HALF_TOWARDS_OWN_GOAL.x,
                        Interpolated::ARGUMENT_FIRST_HALF_OPPONENT_HALF_TOWARDS_OWN_GOAL.x
                    ),
                    0.0
                ),
                Rotation2::new(half_between(
                    Interpolated::ARGUMENT_FIRST_HALF_OWN_HALF_TOWARDS_OWN_GOAL.y,
                    Interpolated::ARGUMENT_FIRST_HALF_OWN_HALF_AWAY_OWN_GOAL.y
                ))
                .into()
            )),
            1.5,
            epsilon = 0.001
        );
    }
}
