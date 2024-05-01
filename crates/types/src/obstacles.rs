use serde::{Deserialize, Serialize};

use linear_algebra::Point2;
use path_serde::{PathDeserialize, PathIntrospect, PathSerialize};

use coordinate_systems::Ground;

#[derive(
    Clone,
    Copy,
    Debug,
    Default,
    Deserialize,
    Serialize,
    PathSerialize,
    PathDeserialize,
    PathIntrospect,
)]
pub enum ObstacleKind {
    Ball,
    GoalPost,
    Robot,
    #[default]
    Unknown,
}

#[derive(
    Clone, Copy, Debug, Deserialize, Serialize, PathSerialize, PathDeserialize, PathIntrospect,
)]
pub struct Obstacle {
    pub kind: ObstacleKind,
    pub position: Point2<Ground>,
    pub radius_at_foot_height: f32,
    pub radius_at_hip_height: f32,
}

impl Obstacle {
    pub fn ball(position: Point2<Ground>, radius: f32) -> Self {
        Self {
            kind: ObstacleKind::Ball,
            position,
            radius_at_foot_height: radius,
            radius_at_hip_height: radius,
        }
    }

    pub fn robot(
        position: Point2<Ground>,
        radius_at_foot_height: f32,
        radius_at_hip_height: f32,
    ) -> Self {
        Self {
            kind: ObstacleKind::Robot,
            position,
            radius_at_foot_height,
            radius_at_hip_height,
        }
    }

    pub fn goal_post(position: Point2<Ground>, radius: f32) -> Self {
        Self {
            kind: ObstacleKind::GoalPost,
            position,
            radius_at_foot_height: radius,
            radius_at_hip_height: radius,
        }
    }
}
