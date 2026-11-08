use std::ops::Mul;

use serde::{Deserialize, Serialize};

use linear_algebra::{Isometry2, Point2};

use coordinate_systems::Ground;

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize, ros_z::Message)]
pub enum ObstacleKind {
    Ball,
    GoalPost,
    Robot,
    Person,
    Unknown,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, ros_z::Message)]
pub struct Obstacle {
    pub kind: ObstacleKind,
    pub position: Point2<Ground>,
    pub radius_at_foot_height: f32,
    pub radius_at_hip_height: f32,
}

/// The same obstacle seen from another ground frame.
impl Mul<Obstacle> for Isometry2<Ground, Ground> {
    type Output = Obstacle;

    fn mul(self, rhs: Obstacle) -> Self::Output {
        Obstacle {
            position: self * rhs.position,
            ..rhs
        }
    }
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
