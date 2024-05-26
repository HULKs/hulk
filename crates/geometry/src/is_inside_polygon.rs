use linear_algebra::Point2;
use nalgebra::Matrix2;

pub fn is_inside_polygon<Frame>(points: &[Point2<Frame>], target_point: &Point2<Frame>) -> bool {
    if !points.is_empty() {
        let mut crossings = 0;
        for i in 0..points.len() {
            let j = (i + 1) % points.len();

            if points[i].y() <= target_point.y() {
                if points[j].y() > target_point.y()
                    && cross_product(target_point, &points[i], &points[j]) > 0.0
                {
                    crossings += 1;
                }
            } else if points[j].y() <= target_point.y()
                && cross_product(target_point, &points[i], &points[j]) < 0.0
            {
                crossings -= 1;
            }
        }

        return crossings != 0;
    }
    false
}

fn cross_product<Frame>(
    point: &Point2<Frame>,
    segment_start: &Point2<Frame>,
    segment_end: &Point2<Frame>,
) -> f32 {
    let point_to_segment_start = *segment_start - *point;
    let point_to_segment_end = *segment_end - *point;
    Matrix2::from_columns(&[point_to_segment_start.inner, point_to_segment_end.inner]).determinant()
}
