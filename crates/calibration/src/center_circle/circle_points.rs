use serde::{Deserialize, Serialize};

use linear_algebra::Point2;

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct CenterCirclePoints<Frame> {
    pub center: Point2<Frame>,
    pub points: Vec<Point2<Frame>>,
}
