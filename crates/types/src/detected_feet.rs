use linear_algebra::Point2;
use serde::{Deserialize, Serialize};

use coordinate_systems::{Ground, Pixel};

#[derive(Default, Clone, Debug, Deserialize, Serialize)]
pub struct DetectedFeet {
    pub positions: Vec<Point2<Ground>>,
}

#[derive(Default, Clone, Debug, Deserialize, Serialize)]
pub struct ClusterPoint {
    pub pixel_coordinates: Point2<Pixel, u16>,
    pub position_in_ground: Point2<Ground>,
}

#[derive(Default, Clone, Debug, Deserialize, Serialize)]
pub struct CountedCluster {
    pub mean: Point2<Ground>,
    pub samples: usize,
    pub leftmost_point: Point2<Ground>,
    pub rightmost_point: Point2<Ground>,
}
