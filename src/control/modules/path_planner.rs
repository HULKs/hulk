use macros::{module, require_some};
use nalgebra::{Isometry2, UnitComplex};

use crate::types::{Motion, MotionCommand, PlannedPath};

pub struct PathPlanner {}

#[module(control)]
#[input(path = motion_command, data_type = MotionCommand)]
#[parameter(path = control.path_planner.hybrid_align_distance, data_type = f32)]
#[parameter(path = control.path_planner.distance_to_be_aligned, data_type = f32)]
#[main_output(data_type = PlannedPath)]
impl PathPlanner {}

impl PathPlanner {
    pub fn new() -> Self {
        Self {}
    }

    fn cycle(&mut self, context: CycleContext) -> anyhow::Result<MainOutputs> {
        let requested_motion = require_some!(context.motion_command).motion;

        let requested_target = match requested_motion {
            Motion::Walk { target_pose, .. } => target_pose,
            _ => Isometry2::identity(),
        };

        let rotation = interpolate_rotation(
            requested_target,
            *context.hybrid_align_distance,
            *context.distance_to_be_aligned,
        );

        let end_pose = Isometry2::from_parts(requested_target.translation, rotation);

        Ok(MainOutputs {
            planned_path: Some(PlannedPath { end_pose }),
        })
    }
}

fn interpolate_rotation(
    target_pose: Isometry2<f32>,
    hybrid_align_distance: f32,
    distance_to_be_aligned: f32,
) -> UnitComplex<f32> {
    assert!(hybrid_align_distance > distance_to_be_aligned);
    let distance_to_target = target_pose.translation.vector.norm();
    if distance_to_target < 0.01 {
        return target_pose.rotation;
    }
    let target_facing_rotation =
        UnitComplex::new(target_pose.translation.y.atan2(target_pose.translation.x));
    let t = ((distance_to_target - distance_to_be_aligned)
        / (hybrid_align_distance - distance_to_be_aligned))
        .clamp(0.0, 1.0);
    target_pose.rotation.slerp(&target_facing_rotation, t)
}

#[cfg(test)]
mod tests {
    use std::f32::consts::{FRAC_PI_2, FRAC_PI_4};

    use approx::assert_relative_eq;
    use nalgebra::vector;

    use super::*;

    #[test]
    fn zero_target_pose_is_not_altered() {
        let target_pose = Isometry2::new(vector![0.0, 0.0], FRAC_PI_2);
        let interpolated_rotation = interpolate_rotation(target_pose, 1.0, 0.05);
        assert_relative_eq!(interpolated_rotation, target_pose.rotation);
    }

    #[test]
    fn near_target_pose_is_fully_aligned() {
        let target_pose = Isometry2::new(vector![0.05, 0.0], FRAC_PI_2);
        let interpolated_rotation = interpolate_rotation(target_pose, 1.0, 0.05);
        assert_relative_eq!(interpolated_rotation, target_pose.rotation);
    }

    #[test]
    fn far_target_pose_is_facing_target() {
        let target_pose = Isometry2::new(vector![5.0, 0.0], FRAC_PI_2);
        let interpolated_rotation = interpolate_rotation(target_pose, 1.0, 0.05);
        assert_relative_eq!(interpolated_rotation, UnitComplex::new(0.0));
    }

    #[test]
    fn middle_target_pose_is_oriented_halfways_facing_target() {
        let target_pose = Isometry2::new(vector![0.525, 0.0], FRAC_PI_2);
        let interpolated_rotation = interpolate_rotation(target_pose, 1.0, 0.05);
        assert_relative_eq!(interpolated_rotation, UnitComplex::new(FRAC_PI_4));
    }
}
