use std::f32::consts::PI;

use coordinate_systems::Ground;
use linear_algebra::{Orientation2, Point2};
use ros_z::Message;
use serde::{Deserialize, Serialize};
use types::{
    motion_command::OrientationMode,
    path::{
        Path,
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
) -> Step {
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

    Step {
        forward: velocity.x(),
        left: velocity.y(),
        turn: angular_velocity,
    }
}

#[cfg(test)]
mod tests {
    use linear_algebra::{Orientation2, point};
    use types::path::direct_path;

    use super::*;

    fn walking_parameters() -> WalkingParameters {
        WalkingParameters {
            hybrid_align_distance: 1.0,
            max_alignment_rate: 2.0,
            deceleration_distance: 0.5,
        }
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
        );

        assert!((step.forward - 0.5).abs() < 0.001);
    }
}
