use geometry::{line::Line2, rectangle::Rectangle};
use serde::{Deserialize, Serialize};

use linear_algebra::Point2;
use path_serde::{PathDeserialize, PathIntrospect, PathSerialize};

#[derive(
    Clone, Debug, Default, Serialize, Deserialize, PathSerialize, PathIntrospect, PathDeserialize,
)]
pub struct CenterCirclePoints<Frame> {
    pub center: Point2<Frame>,
    pub points: Vec<Point2<Frame>>,
    pub bounding_box: Rectangle<Frame>,
}

#[derive(
    Clone, Debug, Default, Serialize, Deserialize, PathSerialize, PathIntrospect, PathDeserialize,
)]
pub struct CenterCirclePointsSeperated<Frame> {
    pub center: Point2<Frame>,
    pub inner_points: Vec<Point2<Frame>>,
    pub outer_points: Vec<Point2<Frame>>,
}

impl<Frame> CenterCirclePoints<Frame> {
    pub fn total_points(&self) -> usize {
        self.points.len()
    }
}

#[derive(
    Clone, Debug, Default, Serialize, Deserialize, PathSerialize, PathIntrospect, PathDeserialize,
)]
pub struct MidLineAndPoints<Frame> {
    pub mid_line: Option<Line2<Frame>>,
    pub points: Vec<Point2<Frame>>,
}
