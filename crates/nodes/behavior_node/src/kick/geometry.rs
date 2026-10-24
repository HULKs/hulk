use coordinate_systems::{Field, Ground};
use geometry::{circle::Circle, line_segment::LineSegment, visibility::visible_segments};
use linear_algebra::{Isometry2, Point2, point, vector};
use types::{
    field_dimensions::{FieldDimensions, Half, Side},
    obstacles::{Obstacle, ObstacleKind},
};

pub fn goal_mouth(field: &FieldDimensions, half: Half) -> LineSegment<Field> {
    let x = match half {
        Half::Own => -field.length / 2.0,
        Half::Opponent => field.length / 2.0,
    };
    LineSegment(
        point![x, -field.goal_inner_width / 2.0],
        point![x, field.goal_inner_width / 2.0],
    )
}

/// Geometrically open opponent-goal segments for the ball center, in field coordinates.
/// `clearance` is an additional nonnegative physical margin in meters, not a risk score.
/// Boundaries may be tangent to inflated obstacles; execution must choose an interior target.
pub fn goal_openings(
    ball: Point2<Field>,
    ground_to_field: Isometry2<Ground, Field>,
    obstacles: &[Obstacle],
    field: &FieldDimensions,
    clearance: f32,
) -> Vec<LineSegment<Field>> {
    let margin = field.ball_radius + clearance;
    let goal_x = field.length / 2.0;
    let half_width = field.goal_inner_width / 2.0 - margin;
    if ball.x() >= goal_x || half_width <= 0.0 {
        return Vec::new();
    }
    let post_radius = field.goal_post_diameter / 2.0;
    let circles: Vec<_> = obstacles
        .iter()
        .filter(|obstacle| obstacle.kind != ObstacleKind::Ball)
        .map(|obstacle| {
            Circle::new(
                ground_to_field * obstacle.position,
                obstacle.radius_at_foot_height + margin,
            )
        })
        .chain([Side::Left, Side::Right].map(|side| {
            Circle::new(
                // Match the obstacle filter's placement behind the goal line.
                field.goal_post(Half::Opponent, side)
                    + vector![post_radius - field.line_width / 2.0, 0.0],
                post_radius + margin,
            )
        }))
        .collect();

    visible_segments(
        ball,
        LineSegment(point![goal_x, -half_width], point![goal_x, half_width]),
        &circles,
    )
}

#[cfg(test)]
mod tests {
    use approx::assert_relative_eq;
    use linear_algebra::IntoTransform;

    use super::*;

    #[test]
    fn goal_openings_transform_obstacles_and_include_ball_clearance() {
        let field = FieldDimensions::SPL_2025;
        let ball = point![0.0, 0.0];
        let transform: Isometry2<Ground, Field> =
            nalgebra::Isometry2::new(nalgebra::vector![1.0, -0.5], 0.7).framed_transform();
        let openings = goal_openings(ball, transform, &[], &field, 0.1);
        assert_eq!(openings.len(), 1);
        assert_relative_eq!(openings[0].0.y(), -0.6);
        assert_relative_eq!(openings[0].1.y(), 0.6);

        let obstacle = Obstacle::robot(transform.inverse() * point![3.0, 0.0], 0.2, 0.4);
        let openings = goal_openings(ball, transform, &[obstacle], &field, 0.1);
        assert_eq!(openings.len(), 2);
        let shadow_half_width = 4.5 * 0.35 / (9.0_f32 - 0.35_f32.powi(2)).sqrt();
        assert_relative_eq!(openings[0].1.y(), -shadow_half_width, epsilon = 1e-6);
        assert_relative_eq!(openings[1].0.y(), shadow_half_width, epsilon = 1e-6);
        let ball_obstacle = Obstacle::ball(transform.inverse() * ball, field.ball_radius);
        assert_eq!(
            goal_openings(ball, transform, &[ball_obstacle], &field, 0.1).len(),
            1
        );
        assert!(
            goal_openings(point![field.length / 2.0, 0.0], transform, &[], &field, 0.1).is_empty()
        );
        assert!(goal_openings(ball, transform, &[], &field, 1.0).is_empty());
    }
}
