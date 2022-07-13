use nalgebra::{vector, Point2, Vector2};
use serde::{Deserialize, Serialize};
use serialize_hierarchy::SerializeHierarchy;

#[derive(Default, Clone, Debug, Deserialize, Serialize, SerializeHierarchy)]
pub struct DetectedRobots {
    pub robot_positions: Vec<ScoredCluster>,
}

#[derive(Default, Clone, Debug, Deserialize, Serialize, SerializeHierarchy)]
pub struct ScoredCluster {
    pub center: Point2<f32>,
    pub score: f32,
}

#[derive(Default, Clone, Debug, Deserialize, Serialize, SerializeHierarchy)]
pub struct ScoredClusterPoint {
    pub point: Point2<f32>,
    pub amount_score: f32,
    pub luminance_score: f32,
}

#[derive(Default, Clone, Debug, Deserialize, Serialize, SerializeHierarchy)]
pub struct ClusterCone {
    pub left: Vector2<f32>,
    pub right: Vector2<f32>,
}

impl ClusterCone {
    pub fn from_cluster(cluster: &ScoredCluster, cluster_cone_radius: f32) -> Self {
        let robot_to_center = cluster.center.coords;
        let unit_robot_to_center = robot_to_center.normalize();
        let center_to_left =
            vector![unit_robot_to_center.y, -unit_robot_to_center.x] * cluster_cone_radius;
        let center_to_right = -center_to_left;
        let robot_to_left = robot_to_center + center_to_left;
        let robot_to_right = robot_to_center + center_to_right;
        Self {
            left: robot_to_left,
            right: robot_to_right,
        }
    }

    pub fn intersects_with(&self, other: &Self) -> bool {
        vector_is_in_between(self.left, self.right, other.left)
            || vector_is_in_between(self.left, self.right, other.right)
    }
}

fn vector_is_in_between(left: Vector2<f32>, right: Vector2<f32>, other: Vector2<f32>) -> bool {
    let angle_between_left_and_right = left.angle(&right);
    let angle_between_left_and_other_left = left.angle(&other);
    let angle_between_right_and_other_left = right.angle(&other);
    angle_between_left_and_other_left <= angle_between_left_and_right
        && angle_between_right_and_other_left <= angle_between_left_and_right
}
