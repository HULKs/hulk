use std::time::{SystemTime, UNIX_EPOCH};

use nalgebra::{Isometry2, Point2, Vector2};
use serde::{Deserialize, Serialize};
use serialize_hierarchy::SerializeHierarchy;
use spl_network_messages::PlayerNumber;

use crate::{
    fall_state::FallState, filtered_game_controller_state::FilteredGameControllerState,
    kick_decision::KickDecision, obstacles::Obstacle, penalty_shot_direction::PenaltyShotDirection,
    primary_state::PrimaryState, roles::Role, rule_obstacles::RuleObstacle, support_foot::Side,
};

#[derive(Clone, Debug, Default, Serialize, Deserialize, SerializeHierarchy)]
pub struct WorldState {
    pub ball: Option<BallState>,
    pub rule_ball: Option<BallState>,
    pub filtered_game_controller_state: Option<FilteredGameControllerState>,
    pub obstacles: Vec<Obstacle>,
    pub rule_obstacles: Vec<RuleObstacle>,
    pub position_of_interest: Point2<f32>,
    pub kick_decisions: Option<Vec<KickDecision>>,
    pub instant_kick_decisions: Option<Vec<KickDecision>>,
    pub robot: RobotState,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, SerializeHierarchy)]
pub struct BallState {
    pub ball_in_ground: Point2<f32>,
    pub ball_in_field: Point2<f32>,
    pub ball_in_ground_velocity: Vector2<f32>,
    pub last_seen_ball: SystemTime,
    pub penalty_shot_direction: Option<PenaltyShotDirection>,
    pub field_side: Side,
}

impl BallState {
    pub fn new_at_center(robot_to_field: Isometry2<f32>) -> Self {
        Self {
            ball_in_field: Point2::origin(),
            ball_in_ground: robot_to_field.inverse() * Point2::origin(),
            ball_in_ground_velocity: Vector2::zeros(),
            last_seen_ball: UNIX_EPOCH,
            penalty_shot_direction: Default::default(),
            field_side: Side::Left,
        }
    }
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, SerializeHierarchy)]
pub struct RobotState {
    pub robot_to_field: Option<Isometry2<f32>>,
    pub role: Role,
    pub primary_state: PrimaryState,
    pub fall_state: FallState,
    pub has_ground_contact: bool,
    pub player_number: PlayerNumber,
}
