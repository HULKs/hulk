use color_eyre::eyre::{Result, ensure};
use std::f32::consts::PI;

use coordinate_systems::Ground;
use linear_algebra::{Orientation2, Point2};
use motion_inference::config::LocomotionParameters;
use ros_z::Message;
use serde::{Deserialize, Serialize};
use types::{
    motion_command::OrientationMode,
    path::{
        Path, PathSegment,
        traits::{Length, PathProgress},
    },
    step::Step,
};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, Message)]
#[serde(deny_unknown_fields)]
pub struct WalkingParameters {
    pub hybrid_align_distance: f32,
    pub max_alignment_rate: f32,
    pub deceleration_distance: f32,
}

pub fn target_alignment_importance(
    distance_to_be_aligned: f32,
    hybrid_align_distance: f32,
    distance_to_target: f32,
) -> f32 {
    if distance_to_target < distance_to_be_aligned {
        1.0
    } else if distance_to_target < distance_to_be_aligned + hybrid_align_distance {
        (1.0 + f32::cos(PI * (distance_to_target - distance_to_be_aligned) / hybrid_align_distance))
            * 0.5
    } else {
        0.0
    }
}

pub fn step_from_walk_command(
    path: &Path,
    orientation_mode: OrientationMode,
    target_orientation: Orientation2<Ground>,
    distance_to_be_aligned: f32,
    speed: f32,
    parameters: &WalkingParameters,
) -> Result<Step> {
    ensure!(!path.segments.is_empty(), "empty walking path");
    ensure!(
        speed.is_finite()
            && speed >= 0.0
            && distance_to_be_aligned.is_finite()
            && distance_to_be_aligned >= 0.0,
        "invalid walking speed or alignment distance"
    );
    for segment in &path.segments {
        let valid = match segment {
            PathSegment::LineSegment(line) => line
                .0
                .inner
                .iter()
                .chain(line.1.inner.iter())
                .all(|v| v.is_finite()),
            PathSegment::Arc(arc) => {
                arc.circle.center.inner.iter().all(|v| v.is_finite())
                    && arc.circle.radius.is_finite()
                    && arc.circle.radius > 0.0
                    && arc
                        .start
                        .as_unit_vector()
                        .inner
                        .iter()
                        .chain(arc.end.as_unit_vector().inner.iter())
                        .all(|v| v.is_finite())
            }
        };
        ensure!(valid, "invalid walking path geometry");
    }
    ensure!(
        target_orientation.angle().is_finite(),
        "invalid target orientation"
    );
    let length = path.length();
    ensure!(
        length.is_finite() && length >= 0.0,
        "invalid walking path length"
    );
    if length <= f32::EPSILON {
        return Ok(Step {
            forward: 0.0,
            left: 0.0,
            turn: target_orientation.as_unit_vector().y() * parameters.max_alignment_rate,
        });
    }
    let forward = path.forward(Point2::origin());
    let distance_to_target = path.length();
    let deceleration_factor =
        (distance_to_target / parameters.deceleration_distance).clamp(0.0, 1.0);
    let velocity = forward * speed * deceleration_factor;

    let walk_orientation = match orientation_mode {
        OrientationMode::Unspecified | OrientationMode::AlignWithPath => {
            Orientation2::from_vector(forward)
        }
        OrientationMode::LookTowards { direction, .. } => direction,
        OrientationMode::LookAt { target, .. } => {
            Orientation2::from_vector(target - Point2::origin())
        }
    };

    let target_alignment_importance = target_alignment_importance(
        distance_to_be_aligned,
        parameters.hybrid_align_distance,
        distance_to_target,
    );

    let orientation = walk_orientation.slerp(target_orientation, target_alignment_importance);
    let angular_velocity = orientation.as_unit_vector().y() * parameters.max_alignment_rate;

    ensure!(
        [velocity.x(), velocity.y(), angular_velocity]
            .into_iter()
            .all(f32::is_finite),
        "invalid walking step"
    );
    Ok(Step {
        forward: velocity.x(),
        left: velocity.y(),
        turn: angular_velocity,
    })
}

/// A path speed is direction independent, but the policy's trained envelope is not.
/// Scale translation uniformly so limiting a sideways/backwards path preserves its tangent.
/// Direct velocity requests bypass this helper and retain inference's strict validation.
pub(super) fn limit_generated_step(step: Step, limits: &LocomotionParameters) -> Result<Step> {
    let [backward, forward] = limits.forward_velocity_limits;
    ensure!(
        [step.forward, step.left, step.turn]
            .into_iter()
            .all(f32::is_finite),
        "invalid generated walking step"
    );
    ensure!(
        [
            backward,
            forward,
            limits.lateral_velocity_limit,
            limits.angular_velocity_limit
        ]
        .into_iter()
        .all(f32::is_finite)
            && backward <= 0.0
            && forward >= 0.0
            && limits.lateral_velocity_limit >= 0.0
            && limits.angular_velocity_limit >= 0.0,
        "walking envelope must be finite and include standing"
    );
    let mut scale = 1.0_f32;
    if step.forward < backward {
        scale = scale.min(backward / step.forward);
    } else if step.forward > forward {
        scale = scale.min(forward / step.forward);
    }
    if step.left.abs() > limits.lateral_velocity_limit {
        scale = scale.min(limits.lateral_velocity_limit / step.left.abs());
    }
    Ok(Step {
        // Clamp boundary roundoff so inference's inclusive envelope check remains exact.
        forward: (step.forward * scale).clamp(backward, forward),
        left: (step.left * scale).clamp(
            -limits.lateral_velocity_limit,
            limits.lateral_velocity_limit,
        ),
        turn: step.turn.clamp(
            -limits.angular_velocity_limit,
            limits.angular_velocity_limit,
        ),
    })
}

#[cfg(test)]
mod tests {
    use linear_algebra::{Orientation2, point, vector};
    use types::{
        motion_command::{HeadMotion, MotionCommand},
        path::direct_path,
    };

    use crate::MotionPlan;

    use super::*;

    fn walking_parameters() -> WalkingParameters {
        WalkingParameters {
            hybrid_align_distance: 1.0,
            max_alignment_rate: 2.0,
            deceleration_distance: 0.5,
        }
    }

    fn locomotion_parameters() -> LocomotionParameters {
        LocomotionParameters {
            forward_velocity_limits: [-1.0, 2.0],
            lateral_velocity_limit: 1.0,
            angular_velocity_limit: 1.5,
            base_frequency: 1.0,
            initial_frequency_offset: 0.0,
            frequency_offset_limit: 1.0,
        }
    }

    #[test]
    fn twice_speed_paths_preserve_direction_within_asymmetric_policy_envelope() {
        let limits = locomotion_parameters();
        for (target, expected) in [
            (point![5.0, 0.0], vector![2.0, 0.0]),
            (point![-5.0, 0.0], vector![-1.0, 0.0]),
            (point![0.0, 5.0], vector![0.0, 1.0]),
            (point![0.0, -5.0], vector![0.0, -1.0]),
            (point![5.0, 5.0], vector![1.0, 1.0]),
            (point![-5.0, 5.0], vector![-1.0, 1.0]),
        ] {
            let request = MotionCommand::Walk {
                head: HeadMotion::ZeroAngles,
                path: direct_path(Point2::origin(), target),
                orientation_mode: OrientationMode::AlignWithPath,
                target_orientation: Orientation2::new(0.0),
                distance_to_be_aligned: 0.0,
                speed: 2.0,
            };
            let MotionPlan::Walk { command, .. } =
                MotionPlan::from_motion_command(&request, &walking_parameters(), &limits).unwrap()
            else {
                panic!("path request did not produce a walking plan");
            };
            assert!((command.velocity - expected).norm() < 1e-6);
            assert!(
                (limits.forward_velocity_limits[0]..=limits.forward_velocity_limits[1])
                    .contains(&command.velocity.x())
            );
            assert!(command.velocity.y().abs() <= limits.lateral_velocity_limit);
            assert!(command.angular_velocity.abs() <= limits.angular_velocity_limit);
        }
    }

    #[test]
    fn generated_step_uses_configured_limits_and_keeps_in_range_requests() {
        let mut limits = locomotion_parameters();
        limits.forward_velocity_limits = [-0.4, 0.8];
        limits.lateral_velocity_limit = 0.3;
        limits.angular_velocity_limit = 0.2;
        let limited = limit_generated_step(
            Step {
                forward: -2.0,
                left: 1.0,
                turn: -1.0,
            },
            &limits,
        )
        .unwrap();
        assert!((limited.forward + 0.4).abs() < 1e-6);
        assert!((limited.left - 0.2).abs() < 1e-6);
        assert_eq!(limited.turn, -0.2);

        let original = Step {
            forward: 0.6,
            left: -0.2,
            turn: 0.1,
        };
        let unchanged = limit_generated_step(original, &limits).unwrap();
        assert_eq!(unchanged.forward, original.forward);
        assert_eq!(unchanged.left, original.left);
        assert_eq!(unchanged.turn, original.turn);
    }

    #[test]
    fn generated_step_rejects_nonfinite_values_and_supports_zero_limits() {
        let mut limits = locomotion_parameters();
        for invalid in [f32::NAN, f32::INFINITY, f32::NEG_INFINITY] {
            for step in [
                Step {
                    forward: invalid,
                    ..Step::ZERO
                },
                Step {
                    left: invalid,
                    ..Step::ZERO
                },
                Step {
                    turn: invalid,
                    ..Step::ZERO
                },
            ] {
                assert!(limit_generated_step(step, &limits).is_err());
            }
        }
        limits.forward_velocity_limits = [0.0, 0.0];
        limits.lateral_velocity_limit = 0.0;
        limits.angular_velocity_limit = 0.0;
        let stopped = limit_generated_step(
            Step {
                forward: -2.0,
                left: 1.0,
                turn: 1.0,
            },
            &limits,
        )
        .unwrap();
        assert_eq!(stopped.forward, 0.0);
        assert_eq!(stopped.left, 0.0);
        assert_eq!(stopped.turn, 0.0);
    }

    #[test]
    fn direct_velocity_requests_remain_unmodified_for_strict_inference_validation() {
        let request = MotionCommand::WalkWithVelocity {
            head: HeadMotion::ZeroAngles,
            velocity: vector![-2.0, 2.0],
            angular_velocity: 3.0,
        };
        let MotionPlan::Walk { command, .. } = MotionPlan::from_motion_command(
            &request,
            &walking_parameters(),
            &locomotion_parameters(),
        )
        .unwrap() else {
            panic!("velocity request did not produce a walking plan");
        };
        assert_eq!(command.velocity, vector![-2.0, 2.0]);
        assert_eq!(command.angular_velocity, 3.0);
    }

    #[test]
    fn target_alignment_importance_is_one_inside_align_distance() {
        assert_eq!(target_alignment_importance(1.0, 2.0, 0.5), 1.0);
    }

    #[test]
    fn target_alignment_importance_is_zero_outside_hybrid_range() {
        assert_eq!(target_alignment_importance(1.0, 2.0, 4.0), 0.0);
    }

    #[test]
    fn target_alignment_importance_blends_with_cosine() {
        let importance = target_alignment_importance(1.0, 2.0, 2.0);
        assert!((importance - 0.5).abs() < f32::EPSILON);
    }

    #[test]
    fn walk_path_decelerates_near_target() {
        let step = step_from_walk_command(
            &direct_path(point![0.0, 0.0], point![0.25, 0.0]),
            OrientationMode::AlignWithPath,
            Orientation2::new(0.0),
            0.0,
            1.0,
            &walking_parameters(),
        )
        .unwrap();

        assert!((step.forward - 0.5).abs() < 0.001);
    }
}
