use linear_algebra::Point2;

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
    seg_start: &Point2<Frame>,
    seg_end: &Point2<Frame>,
) -> f32 {
    (seg_start.x() - point.x()) * (seg_end.y() - point.y())
        - (seg_start.y() - point.y()) * (seg_end.x() - point.x())
}
