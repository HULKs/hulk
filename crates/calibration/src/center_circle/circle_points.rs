use serde::{Deserialize, Serialize};

use linear_algebra::Point2;
use path_serde::{PathDeserialize, PathIntrospect, PathSerialize};

#[derive(
    Clone, Debug, Default, Serialize, Deserialize, PathSerialize, PathIntrospect, PathDeserialize,
)]
pub struct CenterCirclePoints<Frame> {
    pub center: Point2<Frame>,
    pub points: Vec<Point2<Frame>>,
}
