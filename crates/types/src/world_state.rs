use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};

use coordinate_systems::{Field, Ground};
use linear_algebra::{Isometry2, Point2, Vector2};
use path_serde::{PathDeserialize, PathIntrospect, PathSerialize};

use crate::{
    field_dimensions::Side, filtered_game_controller_state::FilteredGameControllerState,
    kick_decision::KickDecision, obstacles::Obstacle, primary_state::PrimaryState,
    rule_obstacles::RuleObstacle,
};

#[derive(Clone, Debug, Serialize, Deserialize, PathSerialize, PathIntrospect)]
pub struct WorldState {
    pub ball: Option<BallState>,
    pub filtered_game_controller_state: Option<FilteredGameControllerState>,
    pub instant_kick_decisions: Option<Vec<KickDecision>>,
    pub kick_decisions: Option<Vec<KickDecision>>,
    pub obstacles: Vec<Obstacle>,
    pub robot: RobotState,
    pub rule_ball: Option<BallState>,
    pub rule_obstacles: Vec<RuleObstacle>,
}

#[allow(clippy::derivable_impls)]
impl Default for WorldState {
    fn default() -> Self {
        Self {
            ball: Default::default(),
            filtered_game_controller_state: Default::default(),
            instant_kick_decisions: Default::default(),
            kick_decisions: Default::default(),
            obstacles: Default::default(),
            robot: Default::default(),
            rule_ball: Default::default(),
            rule_obstacles: Default::default(),
        }
    }
}

#[derive(
    Clone,
    Copy,
    Debug,
    Serialize,
    Deserialize,
    PathSerialize,
    PathDeserialize,
    PathIntrospect,
    PartialEq,
)]
pub struct BallState {
    pub ball_in_ground: Point2<Ground>,
    pub ball_in_field: Point2<Field>,
    pub ball_in_ground_velocity: Vector2<Ground>,
    pub last_seen_ball: SystemTime,
    pub field_side: Side,
}

#[derive(
    Clone,
    Copy,
    Debug,
    Serialize,
    Deserialize,
    PathSerialize,
    PathDeserialize,
    PathIntrospect,
    PartialEq,
)]
pub struct LastBallState {
    pub time: SystemTime,
    pub ball: BallState,
}

impl Default for BallState {
    fn default() -> Self {
        Self {
            ball_in_ground: Point2::origin(),
            ball_in_field: Point2::origin(),
            ball_in_ground_velocity: Vector2::zeros(),
            last_seen_ball: UNIX_EPOCH,
            field_side: Side::Left,
        }
    }
}

impl BallState {
    pub fn new_at_center(ground_to_field: Isometry2<Ground, Field>) -> Self {
        Self {
            ball_in_field: Point2::origin(),
            ball_in_ground: ground_to_field.inverse() * Point2::origin(),
            ball_in_ground_velocity: Vector2::zeros(),
            last_seen_ball: UNIX_EPOCH,
            field_side: Side::Left,
        }
    }
}

#[derive(
    Clone, Debug, Default, Serialize, Deserialize, PathSerialize, PathDeserialize, PathIntrospect,
)]
pub struct RobotState {
    pub ground_to_field: Option<Isometry2<Ground, Field>>,
    pub primary_state: PrimaryState,
    pub role: Role,
}
