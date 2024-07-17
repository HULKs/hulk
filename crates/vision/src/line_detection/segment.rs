use coordinate_systems::Pixel;
use linear_algebra::{point, Point2};
use types::image_segments::EdgeType;

#[derive(Clone, Copy)]
pub struct Segment {
    pub start: Point2<Pixel, u16>,
    pub end: Point2<Pixel, u16>,
    pub start_edge_type: EdgeType,
    pub end_edge_type: EdgeType,
}

impl Segment {
    pub fn center(&self) -> Point2<Pixel, u16> {
        point![
            self.start.x() + (self.end.x() - self.start.x()) / 2,
            self.start.y() + (self.end.y() - self.start.y()) / 2,
        ]
    }
}
