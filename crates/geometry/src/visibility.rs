use linear_algebra::Point2;

use crate::{Distance, circle::Circle, line::Line2, line_segment::LineSegment};

/// Returns visible portions of `target`, ordered from its first endpoint to its second.
/// Paths end at the target: obstacles beyond it only block where they overlap a path.
/// Output endpoints are geometric boundaries and may touch obstacles; aim inside them.
/// Coordinates must be finite and circle radii positive. A degenerate target or an
/// origin on its supporting line has no angular opening and returns no segments.
pub fn visible_segments<Frame: Copy>(
    origin: Point2<Frame>,
    target: LineSegment<Frame>,
    obstacles: &[Circle<Frame>],
) -> Vec<LineSegment<Frame>> {
    let line = Line2::from(target);
    if target.length_squared() == 0.0 || line.distance_to(origin) <= f32::EPSILON {
        return Vec::new();
    }

    let mut boundaries = vec![0.0, 1.0];
    for obstacle in obstacles {
        let Some(tangents) = obstacle.tangents_with_point(origin) else {
            return Vec::new();
        };
        for tangent in [tangents.0, tangents.1] {
            boundaries.push(target.projection_factor(line.intersection(&Line2::from(tangent))));
        }

        // A circle crossing the target can block endpoints before either tangent.
        let center = target.projection_factor(obstacle.center);
        let closest = line.closest_point(obstacle.center);
        let half_chord_squared =
            obstacle.radius.powi(2) - (closest - obstacle.center).norm_squared();
        if half_chord_squared >= 0.0 {
            let offset = (half_chord_squared / target.length_squared()).sqrt();
            boundaries.extend([center - offset, center + offset]);
        }
    }
    boundaries.retain(|value| (0.0..=1.0).contains(value));
    boundaries.sort_unstable_by(f32::total_cmp);
    boundaries.dedup();

    let point_at = |factor| target.0 + (target.1 - target.0) * factor;
    let is_clear = |point| {
        obstacles
            .iter()
            .all(|obstacle| !obstacle.intersects_line_segment(&LineSegment(origin, point)))
    };
    let mut openings: Vec<LineSegment<Frame>> = Vec::new();
    // Visibility is constant between analytic boundaries, so one midpoint suffices.
    // ponytail: O(n²) for small obstacle sets; use a sweep if profiling warrants it.
    for pair in boundaries.windows(2) {
        if !is_clear(point_at((pair[0] + pair[1]) / 2.0)) {
            continue;
        }
        let start = point_at(pair[0]);
        let end = point_at(pair[1]);
        if let Some(previous) = openings.last_mut()
            && (previous.1 - start).norm_squared() == 0.0
            && is_clear(start)
        {
            previous.1 = end;
        } else {
            openings.push(LineSegment(start, end));
        }
    }
    openings
}

#[cfg(test)]
mod tests {
    use approx::assert_relative_eq;
    use coordinate_systems::Field;
    use linear_algebra::point;

    use super::*;

    #[test]
    fn finite_visibility_matches_segment_collision_checks() {
        let origin = point![0.0, 0.0];
        let target = LineSegment::<Field>(point![4.0, -2.0], point![4.0, 2.0]);
        let cases = [
            vec![],
            vec![Circle::new(point![2.0, 0.0], 0.5)],
            vec![Circle::new(point![2.0, 0.0], 1.9)],
            vec![Circle::new(point![-2.0, 0.0], 0.5)],
            vec![Circle::new(point![5.0, 0.0], 0.5)],
            vec![Circle::new(point![4.2, 0.0], 0.5)],
            vec![Circle::new(point![4.0, 0.0], 0.5)],
            vec![Circle::new(point![4.5, 0.0], 0.5)],
            vec![Circle::new(point![0.0, 0.0], 0.5)],
            vec![Circle::new(point![0.5, 0.0], 0.5)],
            vec![Circle::new(point![1.0, 1.0], 1.0)],
            vec![
                Circle::new(point![2.0, -0.3], 0.5),
                Circle::new(point![2.0, 0.3], 0.5),
            ],
        ];
        for obstacles in cases {
            for target in [target, target.flip()] {
                let openings = visible_segments(origin, target, &obstacles);
                for step in 0..200 {
                    let point = target.0 + (target.1 - target.0) * ((step as f32 + 0.5) / 200.0);
                    let expected = obstacles.iter().all(|obstacle| {
                        !obstacle.intersects_line_segment(&LineSegment(origin, point))
                    });
                    let visible = openings
                        .iter()
                        .any(|opening| (0.0..1.0).contains(&opening.projection_factor(point)));
                    assert_eq!(visible, expected, "{point:?}, {obstacles:?}");
                }
            }
        }

        let openings = visible_segments(origin, target, &[Circle::new(point![2.0, 0.0], 0.5)]);
        assert_eq!(openings.len(), 2);
        let shadow_half_width = 4.0 * 0.5 / (4.0_f32 - 0.25).sqrt();
        assert_relative_eq!(openings[0].1.y(), -shadow_half_width);
        assert_relative_eq!(openings[1].0.y(), shadow_half_width);
        assert_eq!(visible_segments(origin, target, &[]).len(), 1);
        assert!(visible_segments(target.0, target, &[]).is_empty());
        assert!(visible_segments(origin, LineSegment(target.0, target.0), &[]).is_empty());
    }
}
