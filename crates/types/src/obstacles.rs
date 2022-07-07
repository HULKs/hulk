use nalgebra::Point2;
use serde::{Deserialize, Serialize};
use serialize_hierarchy::SerializeHierarchy;

use super::Circle;

#[derive(Debug, Clone, Copy, Deserialize, Serialize)]
pub enum ObstacleKind {
    Ball,
    GoalPost,
    Robot,
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, SerializeHierarchy)]
pub struct Obstacle {
    #[leaf]
    pub kind: ObstacleKind,
    pub shape: Circle,
}

impl Obstacle {
    pub fn ball(position: Point2<f32>, radius: f32) -> Self {
        Self {
            shape: Circle {
                center: position,
                radius,
            },
            kind: ObstacleKind::Ball,
        }
    }

    pub fn robot(position: Point2<f32>, radius: f32) -> Self {
        Self {
            shape: Circle {
                center: position,
                radius,
            },
            kind: ObstacleKind::Robot,
        }
    }

    pub fn goal_post(position: Point2<f32>, radius: f32) -> Self {
        Self {
            shape: Circle {
                center: position,
                radius,
            },
            kind: ObstacleKind::GoalPost,
        }
    }
}
