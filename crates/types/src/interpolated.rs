use nalgebra::{matrix, point, Point2};
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
    pub fn evaluate_at(&self, argument: Point2<f32>) -> f32 {
        const ARGUMENT_FIRST_HALF_OWN_HALF_TOWARDS_OWN_GOAL: Point2<f32> = point![0.0, 0.0];
        const ARGUMENT_FIRST_HALF_OWN_HALF_AWAY_OWN_GOAL: Point2<f32> = point![0.0, 1.0];
        const ARGUMENT_FIRST_HALF_OPPONENT_HALF_TOWARDS_OWN_GOAL: Point2<f32> = point![1.0, 0.0];
        const ARGUMENT_FIRST_HALF_OPPONENT_HALF_AWAY_OWN_GOAL: Point2<f32> = point![1.0, 1.0];

        assert_eq!(
            ARGUMENT_FIRST_HALF_OWN_HALF_TOWARDS_OWN_GOAL.x,
            ARGUMENT_FIRST_HALF_OWN_HALF_AWAY_OWN_GOAL.x,
        );
        assert_eq!(
            ARGUMENT_FIRST_HALF_OPPONENT_HALF_TOWARDS_OWN_GOAL.x,
            ARGUMENT_FIRST_HALF_OPPONENT_HALF_AWAY_OWN_GOAL.x,
        );
        assert_eq!(
            ARGUMENT_FIRST_HALF_OWN_HALF_TOWARDS_OWN_GOAL.y,
            ARGUMENT_FIRST_HALF_OPPONENT_HALF_TOWARDS_OWN_GOAL.y,
        );
        assert_eq!(
            ARGUMENT_FIRST_HALF_OWN_HALF_AWAY_OWN_GOAL.y,
            ARGUMENT_FIRST_HALF_OPPONENT_HALF_AWAY_OWN_GOAL.y,
        );

        let x1 = self.first_half_own_half_towards_own_goal;
        let x2 = self.first_half_opponent_half_towards_own_goal;
        let y1 = self.first_half_own_half_towards_own_goal;
        let y2 = self.first_half_own_half_away_own_goal;

        let factor = 1.0 / ((x2 - x1) * (y2 - y1));
        let evaluated_parameters = matrix![
            self.first_half_own_half_towards_own_goal,
            self.first_half_opponent_half_towards_own_goal,
            self.first_half_own_half_away_own_goal,
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
