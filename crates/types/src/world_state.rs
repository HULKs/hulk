use std::time::{SystemTime, UNIX_EPOCH};

use booster::FallDownState;
use hsl_network_messages::{PlayerNumber, PlayerState};
use serde::{Deserialize, Serialize};

use coordinate_systems::{Field, Ground};
use linear_algebra::{Isometry2, Point2, Vector2};
use path_serde::{PathDeserialize, PathIntrospect, PathSerialize};

use crate::{
    ball_position::HypotheticalBallPosition, field_dimensions::Side,
    filtered_game_controller_state::FilteredGameControllerState, obstacles::Obstacle,  players::Players,
    primary_state::PrimaryState, rule_obstacles::RuleObstacle,
};

#[derive(Clone, Debug, Serialize, Deserialize, PathSerialize, PathIntrospect)]
pub struct WorldState {
    pub ball: Option<BallState>,
    pub filtered_game_controller_state: Option<FilteredGameControllerState>,
    pub hypothetical_ball_positions: Vec<HypotheticalBallPosition<Ground>>,
    pub now: SystemTime,
    pub obstacles: Vec<Obstacle>,
    pub player_states: Players<PlayerState>,
    pub position_of_interest: Point2<Ground>,
    pub robot: RobotState,
    pub rule_ball: Option<BallState>,
    pub rule_obstacles: Vec<RuleObstacle>,
    pub fall_down_state: Option<FallDownState>,
    pub suggested_search_position: Option<Point2<Field>>,
}

#[allow(clippy::derivable_impls)]
impl Default for WorldState {
    fn default() -> Self {
        Self {
            ball: Default::default(),
            filtered_game_controller_state: Default::default(),
            hypothetical_ball_positions: Default::default(),
            now: UNIX_EPOCH,
            obstacles: Default::default(),
            player_states: Default::default(),
            position_of_interest: Point2::origin(),
            robot: Default::default(),
            rule_ball: Default::default(),
            rule_obstacles: Default::default(),
            fall_down_state: Default::default(),
            suggested_search_position: Default::default(),
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
    pub player_number: PlayerNumber,
    pub primary_state: PrimaryState,
}
