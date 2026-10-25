use coordinate_systems::{Field, Ground};
use geometry::{
    circle::Circle, line_segment::LineSegment, rectangle::Rectangle, visibility::visible_segments,
};
use linear_algebra::{Isometry2, Point2, point, vector};
use types::{
    field_dimensions::{FieldDimensions, Half, Side},
    obstacles::{Obstacle, ObstacleKind},
    parameters::PassingParameters,
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

// Coordinate round trips can put a ball on the line a few ulps outside it.
pub(super) const BALL_FIELD_BOUNDARY_TOLERANCE: f32 = 1e-4;

/// Safe in-field corridors covering at least `travel_distance` from the ball.
pub fn travel_openings(
    ball: Point2<Field>,
    ground_to_field: Isometry2<Ground, Field>,
    obstacles: &[Obstacle],
    field: &FieldDimensions,
    travel_distance: f32,
    clearance: f32,
) -> Vec<LineSegment<Field>> {
    let margin = field.ball_radius + clearance;
    let half_length = field.length / 2.0 - margin;
    let half_width = field.width / 2.0 - margin;
    if !travel_distance.is_finite()
        || travel_distance <= 0.0
        || half_length <= 0.0
        || half_width <= 0.0
    {
        return Vec::new();
    }
    let bounds = Rectangle {
        min: point![-half_length, -half_width],
        max: point![half_length, half_width],
    };
    let left = ball.x() - travel_distance;
    let right = ball.x() + travel_distance;
    let bottom = ball.y() - travel_distance;
    let top = ball.y() + travel_distance;
    let min_x = left.max(-half_length);
    let max_x = right.min(half_length);
    let min_y = bottom.max(-half_width);
    let max_y = top.min(half_width);
    let circles = obstacle_circles(ground_to_field, obstacles, field, clearance);
    // shortcut: square corridors check up to sqrt(2) times the bound; use arcs if too restrictive.
    [
        LineSegment(point![right, min_y], point![right, max_y]),
        LineSegment(point![min_x, top], point![max_x, top]),
        LineSegment(point![left, min_y], point![left, max_y]),
        LineSegment(point![min_x, bottom], point![max_x, bottom]),
    ]
    .into_iter()
    .filter(|edge| bounds.contains(edge.0) && bounds.contains(edge.1))
    .flat_map(|edge| visible_segments(ball, edge, &circles))
    .collect()
}

/// Ball-center inset leaving room for the receiver's standoff.
pub fn pass_reception_margin(
    field: &FieldDimensions,
    parameters: &PassingParameters,
    clearance: f32,
) -> f32 {
    field.ball_radius + clearance + parameters.receiver_standoff
}

/// Visible reception regions within reach of the receiver, with room to stop in-field.
/// Keep every obstacle, including the receiver, in the finite corridor check.
pub fn pass_openings(
    ball: Point2<Field>,
    receiver: Point2<Field>,
    ground_to_field: Isometry2<Ground, Field>,
    obstacles: &[Obstacle],
    field: &FieldDimensions,
    parameters: &PassingParameters,
    clearance: f32,
) -> Vec<LineSegment<Field>> {
    let margin = field.ball_radius + clearance;
    let reception_margin = pass_reception_margin(field, parameters, clearance);
    let half_length = field.length / 2.0;
    let half_width = field.width / 2.0;
    let delta = receiver - ball;
    let distance = delta.norm();
    if parameters.validate().is_err()
        || !ball
            .inner
            .iter()
            .chain(receiver.inner.iter())
            .all(|v| v.is_finite())
        || !distance.is_finite()
        || distance <= parameters.receiver_standoff
        || !half_length.is_finite()
        || !half_width.is_finite()
        || !reception_margin.is_finite()
        || margin < 0.0
        || half_length <= reception_margin
        || half_width <= reception_margin
        || ball.x().abs() > half_length + BALL_FIELD_BOUNDARY_TOLERANCE
        || ball.y().abs() > half_width + BALL_FIELD_BOUNDARY_TOLERANCE
    {
        return Vec::new();
    }
    let center = receiver - delta / distance * parameters.receiver_standoff;
    let center = point![
        center.x().clamp(
            -half_length + reception_margin,
            half_length - reception_margin
        ),
        center.y().clamp(
            -half_width + reception_margin,
            half_width - reception_margin
        )
    ];
    let travel = (center - ball).norm();
    if travel <= f32::EPSILON
        || travel < parameters.minimum_distance
        || travel >= parameters.maximum_distance
    {
        return Vec::new();
    }
    let direction = (center - ball) / travel;
    let lateral = vector![-direction.y(), direction.x()];
    let half_width = parameters
        .target_half_width
        .min((parameters.maximum_distance.powi(2) - travel.powi(2)).sqrt());
    // An inset target must remain reachable under the receiver's announced-pass acceptance.
    let reach = parameters.receiver_standoff + 2.0 * parameters.target_half_width;
    let offset = receiver - center;
    let lateral_reach_squared = reach.powi(2) - offset.dot(&direction).powi(2);
    if lateral_reach_squared <= 0.0 {
        return Vec::new();
    }
    let lateral_reach = lateral_reach_squared.sqrt();
    let mut lower = (-half_width).max(offset.dot(&lateral) - lateral_reach);
    let mut upper = half_width.min(offset.dot(&lateral) + lateral_reach);
    // shortcut: reserve 20% travel (at least standoff) until kick stopping ranges are calibrated.
    let runway_scale = 1.0 + 0.2_f32.max(parameters.receiver_standoff / travel);
    // Only targets and overshoot endpoints use insets; boundary restarts can enter from the line.
    for (center, lateral, inset) in [
        (center, lateral, reception_margin),
        (
            ball + (center - ball) * runway_scale,
            lateral * runway_scale,
            margin,
        ),
    ] {
        for (coordinate, axis, minimum, maximum) in [
            (
                center.x(),
                lateral.x(),
                -field.length / 2.0 + inset,
                field.length / 2.0 - inset,
            ),
            (
                center.y(),
                lateral.y(),
                -field.width / 2.0 + inset,
                field.width / 2.0 - inset,
            ),
        ] {
            if axis.abs() <= f32::EPSILON {
                if !(minimum..=maximum).contains(&coordinate) {
                    return Vec::new();
                }
            } else {
                let first = (minimum - coordinate) / axis;
                let second = (maximum - coordinate) / axis;
                lower = lower.max(first.min(second));
                upper = upper.min(first.max(second));
            }
            if lower >= upper {
                return Vec::new();
            }
        }
    }
    visible_segments(
        ball,
        LineSegment(center + lateral * lower, center + lateral * upper),
        &obstacle_circles(ground_to_field, obstacles, field, clearance),
    )
}

fn obstacle_circles(
    ground_to_field: Isometry2<Ground, Field>,
    obstacles: &[Obstacle],
    field: &FieldDimensions,
    clearance: f32,
) -> Vec<Circle<Field>> {
    let margin = field.ball_radius + clearance;
    let post_radius = field.goal_post_diameter / 2.0;
    obstacles
        .iter()
        .filter(|obstacle| obstacle.kind != ObstacleKind::Ball)
        .map(|obstacle| {
            Circle::new(
                ground_to_field * obstacle.position,
                obstacle.radius_at_foot_height + margin,
            )
        })
        .chain([Half::Own, Half::Opponent].into_iter().flat_map(|half| {
            [Side::Left, Side::Right].map(move |side| {
                let post = field.goal_post(half, side);
                Circle::new(
                    // Match the obstacle filter's placement behind the goal line.
                    post + vector![
                        post.x().signum() * (post_radius - field.line_width / 2.0),
                        0.0
                    ],
                    post_radius + margin,
                )
            })
        }))
        .collect()
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
    #[test]
    fn pass_regions_respect_field_bounds_and_range_in_any_direction() {
        let field = FieldDimensions::SPL_2025;
        let parameters = PassingParameters {
            maximum_distance: 3.0,
            receiver_standoff: 0.5,
            target_half_width: 0.2,
            ..Default::default()
        };
        for (ball, receiver) in [
            (point![0.0, 2.89], point![2.0, 2.99]),
            (point![0.0, 0.0], point![3.499, 0.0]),
        ] {
            let openings = pass_openings(
                ball,
                receiver,
                Default::default(),
                &[],
                &field,
                &parameters,
                0.05,
            );
            assert_eq!(openings.len(), 1);
            assert!(openings[0].length() < 2.0 * parameters.target_half_width);
            for target in [openings[0].0, openings[0].center(), openings[0].1] {
                assert!((target - ball).norm() <= parameters.maximum_distance + 1e-6);
                let margin = pass_reception_margin(&field, &parameters, 0.05);
                assert!(target.x().abs() <= field.length / 2.0 - margin + 1e-6);
                assert!(target.y().abs() <= field.width / 2.0 - margin + 1e-6);
            }
        }
        for receiver in [point![-2.0, 0.0], point![0.0, -2.0]] {
            assert!(
                !pass_openings(
                    point![0.0, 0.0],
                    receiver,
                    Default::default(),
                    &[],
                    &field,
                    &parameters,
                    0.05
                )
                .is_empty()
            );
        }
        let parameters = PassingParameters {
            minimum_distance: 2.0,
            ..parameters
        };
        for (receiver_x, allowed) in [(2.49, false), (2.5, true), (2.51, true)] {
            let openings = pass_openings(
                Point2::origin(),
                point![receiver_x, 0.0],
                Default::default(),
                &[],
                &field,
                &parameters,
                0.05,
            );
            assert_eq!(!openings.is_empty(), allowed);
            for opening in openings {
                for target in [opening.0, opening.center(), opening.1] {
                    assert!(target.coords().norm() >= parameters.minimum_distance);
                }
            }
        }
    }

    #[test]
    fn sideline_passes_move_inward_and_reserve_runway_without_blocking_restarts() {
        let field = FieldDimensions {
            length: 22.0,
            width: 14.0,
            ball_radius: 0.11,
            ..FieldDimensions::SPL_2025
        };
        let parameters = PassingParameters {
            maximum_distance: 14.0,
            minimum_distance: 4.0,
            receiver_standoff: 0.5,
            target_half_width: 0.3,
            ..Default::default()
        };
        let margin = pass_reception_margin(&field, &parameters, 0.05);
        for side in [-1.0, 1.0] {
            for (ball, receiver) in [
                (point![-5.0, side * 6.9], point![6.0, side * 6.9]),
                (point![0.0, side * 7.0], point![5.0, side * 4.0]),
                (point![11.0, side * 7.0], point![7.0, side * 4.0]),
                (point![0.0, 0.0], point![-0.3, side * 5.0]),
                (point![-7.0, 0.0], point![6.5, side * 0.2]),
            ] {
                let openings = pass_openings(
                    ball,
                    receiver,
                    Default::default(),
                    &[],
                    &field,
                    &parameters,
                    0.05,
                );
                assert!(!openings.is_empty());
                for opening in openings {
                    for target in [opening.0, opening.center(), opening.1] {
                        assert!(target.x().abs() <= field.length / 2.0 - margin + 1e-6);
                        assert!(target.y().abs() <= field.width / 2.0 - margin + 1e-6);
                        assert!(
                            (target - receiver).norm()
                                <= parameters.receiver_standoff
                                    + 2.0 * parameters.target_half_width
                                    + 1e-6
                        );
                        let travel = (target - ball).norm();
                        let overshoot = target
                            + (target - ball) / travel
                                * parameters.receiver_standoff.max(0.2 * travel);
                        assert!(overshoot.x().abs() <= field.length / 2.0 - 0.16 + 1e-6);
                        assert!(overshoot.y().abs() <= field.width / 2.0 - 0.16 + 1e-6);
                    }
                }
            }
            assert!(
                pass_openings(
                    point![-4.0, 0.0],
                    point![2.5, side * 6.8],
                    Default::default(),
                    &[],
                    &field,
                    &parameters,
                    0.05,
                )
                .is_empty()
            );
        }

        // Beyond-target obstacles remain outside the finite delivery corridor.
        let ball = point![0.0, 0.0];
        let receiver = point![5.0, 0.0];
        assert!(
            !pass_openings(
                ball,
                receiver,
                Default::default(),
                &[Obstacle::robot(point![5.5, 0.0], 0.2, 0.2)],
                &field,
                &parameters,
                0.05,
            )
            .is_empty()
        );
        assert!(
            pass_openings(
                ball,
                receiver,
                Default::default(),
                &[Obstacle::robot(point![2.0, 0.0], 1.0, 1.0)],
                &field,
                &parameters,
                0.05,
            )
            .is_empty()
        );
    }
}
