use geometry::{arc::Arc, direction::AngleTo};
use linear_algebra::{Orientation2, Point2};

pub trait ClassifyProjection<Frame> {
    type ProjectionKind;

    fn classify_point(&self, point: Point2<Frame>) -> Self::ProjectionKind;
}

impl<Frame> ClassifyProjection<Frame> for Arc<Frame> {
    type ProjectionKind = ArcProjectionKind;

    fn classify_point(&self, point: Point2<Frame>) -> ArcProjectionKind {
        let center_to_point = point - self.circle.center;
        let angle = center_to_point.y().atan2(center_to_point.x());

        let angle_to_end = self.start.angle_to(self.end, self.direction);
        let angle_to_point = self
            .start
            .angle_to(Orientation2::new(angle), self.direction);

        let is_between_start_and_end = angle_to_point < angle_to_end;

        if is_between_start_and_end {
            ArcProjectionKind::OnArc
        } else {
            let start_point = self.circle.point_at_angle(self.start);
            let end_point = self.circle.point_at_angle(self.end);

            let start_to_point = point - start_point;
            let end_to_point = point - end_point;

            let squared_distance_to_start = start_to_point.inner.norm_squared();
            let squared_distance_to_end = end_to_point.inner.norm_squared();

            if squared_distance_to_start < squared_distance_to_end {
                ArcProjectionKind::Start
            } else {
                ArcProjectionKind::End
            }
        }
    }
}

pub enum ArcProjectionKind {
    OnArc,
    Start,
    End,
}
