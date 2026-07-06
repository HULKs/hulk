use std::f32::consts::PI;

use linear_algebra::{Orientation2, Point2};
use ros_z::Message;
use serde::{Deserialize, Serialize};
use types::{
    motion_command::{MotionCommand, OrientationMode},
    path::traits::{Length, PathProgress},
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

pub fn step_from_motion_command(command: &MotionCommand, parameters: &WalkingParameters) -> Step {
    match command {
        MotionCommand::Walk {
            path,
            orientation_mode,
            target_orientation,
            distance_to_be_aligned,
            speed,
            ..
        } => {
            let forward = path.forward(Point2::origin());
            let distance_to_target = path.length();
            let deceleration_factor =
                (distance_to_target / parameters.deceleration_distance).clamp(0.0, 1.0);
            let velocity = forward * *speed * deceleration_factor;

            let walk_orientation = match orientation_mode {
                OrientationMode::Unspecified | OrientationMode::AlignWithPath => {
                    Orientation2::from_vector(forward)
                }
                OrientationMode::LookTowards { direction, .. } => *direction,
                OrientationMode::LookAt { target, .. } => {
                    Orientation2::from_vector(*target - Point2::origin())
                }
            };

            let target_alignment_importance = target_alignment_importance(
                *distance_to_be_aligned,
                parameters.hybrid_align_distance,
                distance_to_target,
            );

            let orientation =
                walk_orientation.slerp(*target_orientation, target_alignment_importance);
            let angular_velocity = orientation.as_unit_vector().y() * parameters.max_alignment_rate;

            Step {
                forward: velocity.x(),
                left: velocity.y(),
                turn: angular_velocity,
            }
        }
        MotionCommand::WalkWithVelocity {
            velocity,
            angular_velocity,
            ..
        } => Step {
            forward: velocity.x(),
            left: velocity.y(),
            turn: *angular_velocity,
        },
        MotionCommand::Stand { .. }
        | MotionCommand::Damping
        | MotionCommand::Prepare
        | MotionCommand::StandUp
        | MotionCommand::VisualKick { .. } => Step::ZERO,
    }
}

#[cfg(test)]
mod tests {
    use linear_algebra::{Orientation2, point, vector};
    use types::{motion_command::HeadMotion, path::direct_path};

    use super::*;

    fn walking_parameters() -> WalkingParameters {
        WalkingParameters {
            hybrid_align_distance: 1.0,
            max_alignment_rate: 2.0,
            deceleration_distance: 0.5,
        }
    }

    #[test]
    fn walk_with_velocity_maps_directly_to_step() {
        let command = MotionCommand::WalkWithVelocity {
            head: HeadMotion::ZeroAngles,
            velocity: vector![0.3, -0.2],
            angular_velocity: 0.4,
        };

        let step = step_from_motion_command(&command, &walking_parameters());

        assert_eq!(step.forward, 0.3);
        assert_eq!(step.left, -0.2);
        assert_eq!(step.turn, 0.4);
    }

    #[test]
    fn stand_maps_to_zero_step() {
        let command = MotionCommand::Stand {
            head: HeadMotion::ZeroAngles,
        };

        let step = step_from_motion_command(&command, &walking_parameters());

        assert_eq!(step.forward, 0.0);
        assert_eq!(step.left, 0.0);
        assert_eq!(step.turn, 0.0);
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
        let command = MotionCommand::Walk {
            head: HeadMotion::ZeroAngles,
            path: direct_path(point![0.0, 0.0], point![0.25, 0.0]),
            orientation_mode: OrientationMode::AlignWithPath,
            target_orientation: Orientation2::new(0.0),
            distance_to_be_aligned: 0.0,
            speed: 1.0,
        };

        let step = step_from_motion_command(&command, &walking_parameters());

        assert!((step.forward - 0.5).abs() < 0.001);
    }
}
