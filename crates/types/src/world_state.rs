use std::time::{Duration, SystemTime, UNIX_EPOCH};

use hsl_network_messages::PassIntent;
use ros_z::{Message, time::Time};
use serde::{Deserialize, Serialize};

use coordinate_systems::{Field, Ground};
use linear_algebra::{Isometry2, Point2, Pose2, Vector2};

use crate::{
    ball_position::{BallPosition, HypotheticalBallPosition},
    fall_detection::FallDetection,
    field_dimensions::Side,
    filtered_game_controller_state::FilteredGameControllerState,
    obstacles::Obstacle,
    players::Players,
    primary_state::PrimaryState,
    rule_obstacles::RuleObstacle,
    time_wrapper::TimeWrapper,
    tracked_objects::{HumanoidTrack, TrackedObjects},
};

#[derive(Clone, Debug, Default, Serialize, Deserialize, Message)]
pub struct WorldState {
    pub ball: Option<BallState>,
    pub filtered_game_controller_state: Option<FilteredGameControllerState>,
    pub hypothetical_ball_positions: Vec<HypotheticalBallPosition<Ground>>,
    pub humanoid_tracks: Option<TrackedObjects<HumanoidTrack>>,
    pub now: Time,
    pub obstacles: Vec<Obstacle>,
    pub player_states: Players<Option<TimeWrapper<PlayerState>>>,
    pub position_of_interest: Point2<Ground>,
    pub robot: RobotState,
    pub rule_ball: Option<BallState>,
    pub rule_obstacles: Vec<RuleObstacle>,
    pub fall_detection: Option<FallDetection>,
    pub suggested_search_position: Option<Point2<Field>>,
}

#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize, PartialEq, ros_z::Message)]
pub enum BallSource {
    #[default]
    Own,
    Team,
    Rule,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, ros_z::Message)]
pub struct BallState {
    pub ball_in_ground: Point2<Ground>,
    pub ball_in_field: Point2<Field>,
    pub ball_in_ground_velocity: Vector2<Ground>,
    pub last_seen_ball: SystemTime,
    pub field_side: Side,
    pub source: BallSource,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, ros_z::Message)]
pub struct LastBallState {
    pub time: SystemTime,
    pub ball: BallState,
}

impl BallState {
    pub fn new_at_center(ground_to_field: Isometry2<Ground, Field>) -> Self {
        Self {
            ball_in_field: Point2::origin(),
            ball_in_ground: ground_to_field.inverse() * Point2::origin(),
            ball_in_ground_velocity: Vector2::zeros(),
            last_seen_ball: UNIX_EPOCH,
            field_side: Side::Left,
            source: BallSource::Rule,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, Message)]
pub struct RobotState {
    pub ground_to_field: Option<Isometry2<Ground, Field>>,
    pub primary_state: PrimaryState,
}

impl Default for RobotState {
    fn default() -> Self {
        Self {
            ground_to_field: None,
            primary_state: PrimaryState::Damping,
        }
    }
}

#[derive(Copy, Clone, Debug, Serialize, Deserialize, Message)]
pub struct PlayerState {
    pub pose: Pose2<Field>,
    pub head_yaw: f32,
    /// Age of the pose estimate when the teammate sent it, relative to the message's receipt.
    pub pose_age: Option<Duration>,
    pub ball_position: Option<BallPosition<Field>>,
    pub pass: Option<PassIntent>,
    #[serde(default)]
    pub replacement_goalkeeper_claim: bool,
}

impl PlayerState {
    pub fn is_finite(&self) -> bool {
        self.pose.position().x().is_finite()
            && self.pose.position().y().is_finite()
            && self.pose.angle().is_finite()
            && self.head_yaw.is_finite()
            && self
                .ball_position
                .is_none_or(|ball| ball.position.x().is_finite() && ball.position.y().is_finite())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn center_ball_is_rule_generated() {
        let ball = BallState::new_at_center(Isometry2::<Ground, Field>::identity());
        assert_eq!(ball.source, BallSource::Rule);
    }
}
