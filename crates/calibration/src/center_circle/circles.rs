use linear_algebra::Point2;

#[derive(Clone)]
pub struct CenterOfCircleAndPoints<Frame> {
    pub center: Point2<Frame>,
    pub points: Vec<Point2<Frame>>,
}
