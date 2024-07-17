use linear_algebra::Point2;
use path_serde::{PathDeserialize, PathIntrospect, PathSerialize};
use serde::{Deserialize, Serialize};

use coordinate_systems::{Ground, Pixel};

#[derive(
    Default, Clone, Debug, Deserialize, Serialize, PathSerialize, PathDeserialize, PathIntrospect,
)]
pub struct DetectedFeet {
    pub positions: Vec<Point2<Ground>>,
}

#[derive(
    Default, Clone, Debug, Deserialize, Serialize, PathSerialize, PathDeserialize, PathIntrospect,
)]
pub struct ClusterPoint {
    pub pixel_coordinates: Point2<Pixel, u16>,
    pub position_in_ground: Point2<Ground>,
}

#[derive(
    Default, Clone, Debug, Deserialize, Serialize, PathSerialize, PathDeserialize, PathIntrospect,
)]
pub struct CountedCluster {
    pub mean: Point2<Ground>,
    pub samples: usize,
    pub leftmost_point: Point2<Ground>,
    pub rightmost_point: Point2<Ground>,
}
