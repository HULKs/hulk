use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};

use coordinate_systems::{Field, Ground, UpcomingSupport};
use linear_algebra::{Isometry2, Point2, Vector2};
use path_serde::{PathDeserialize, PathIntrospect, PathSerialize};

use crate::{
    ball_position::HypotheticalBallPosition, calibration::CalibrationCommand,
    fall_state::FallState, field_dimensions::Side,
    filtered_game_controller_state::FilteredGameControllerState, kick_decision::KickDecision,
    obstacles::Obstacle, penalty_shot_direction::PenaltyShotDirection, primary_state::PrimaryState,
    roles::Role, rule_obstacles::RuleObstacle,
};

#[derive(Clone, Debug, Default, Serialize, Deserialize, PathSerialize, PathIntrospect)]
pub struct WorldState {
    pub ball: Option<BallState>,
    pub rule_ball: Option<BallState>,
    pub hypothetical_ball_positions: Vec<HypotheticalBallPosition<Ground>>,
    pub filtered_game_controller_state: Option<FilteredGameControllerState>,
    pub obstacles: Vec<Obstacle>,
    pub rule_obstacles: Vec<RuleObstacle>,
    pub position_of_interest: Point2<Ground>,
    pub suggested_search_position: Option<Point2<Field>>,
    pub kick_decisions: Option<Vec<KickDecision>>,
    pub instant_kick_decisions: Option<Vec<KickDecision>>,
    pub robot: RobotState,
    pub calibration_command: Option<CalibrationCommand>,
    pub walk_in_position_index: usize,
}

#[derive(
    Clone, Copy, Debug, Serialize, Deserialize, PathSerialize, PathDeserialize, PathIntrospect,
)]
pub struct BallState {
    pub ball_in_ground: Point2<Ground>,
    pub ball_in_field: Point2<Field>,
    pub ball_in_ground_velocity: Vector2<Ground>,
    pub last_seen_ball: SystemTime,
    pub penalty_shot_direction: Option<PenaltyShotDirection>,
    pub field_side: Side,
}

impl Default for BallState {
    fn default() -> Self {
        Self {
            ball_in_ground: Point2::origin(),
            ball_in_field: Point2::origin(),
            ball_in_ground_velocity: Vector2::zeros(),
            last_seen_ball: UNIX_EPOCH,
            penalty_shot_direction: Default::default(),
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
            penalty_shot_direction: Default::default(),
            field_side: Side::Left,
        }
    }
}

#[derive(
    Clone, Debug, Default, Serialize, Deserialize, PathSerialize, PathDeserialize, PathIntrospect,
)]
pub struct RobotState {
    pub ground_to_field: Option<Isometry2<Ground, Field>>,
    pub role: Role,
    pub primary_state: PrimaryState,
    pub fall_state: FallState,
    pub has_ground_contact: bool,
    pub jersey_number: usize,
    pub ground_to_upcoming_support: Isometry2<Ground, UpcomingSupport>,
}
