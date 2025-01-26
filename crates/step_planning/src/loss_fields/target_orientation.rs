use linear_algebra::Point2;
use types::planned_path::Path;

use crate::{
    geometry::{angle::Angle, Pose},
    traits::{Length, LossField, PathProgress},
    utils::{angle_penalty, angle_penalty_derivative},
};

pub struct TargetOrientationField<'a> {
    pub target_orientation: Angle<f32>,
    pub path: &'a Path,
    pub alignment_start_distance: f32,
    pub ramp_width: f32,
}

impl LossField for TargetOrientationField<'_> {
    type Parameter = Pose<f32>;
    type Gradient = Pose<f32>;
    type Loss = f32;

    fn loss(&self, pose: Self::Parameter) -> Self::Loss {
        let progress = self.path.progress(pose.position);
        let path_length = self.path.length();

        let distance_to_target = path_length - progress;

        angle_penalty(Angle(pose.orientation), self.target_orientation)
            * self.importance(distance_to_target)
    }

    fn grad(&self, pose: Self::Parameter) -> Self::Gradient {
        let progress = self.path.progress(pose.position);
        let path_length = self.path.length();

        let distance_to_target = path_length - progress;

        Pose {
            position: Point2::origin(),
            orientation: angle_penalty_derivative(Angle(pose.orientation), self.target_orientation)
                .into_inner(),
        } * self.importance_derivative(distance_to_target)
    }
}

impl TargetOrientationField<'_> {
    fn importance(&self, distance_to_target: f32) -> f32 {
        // if distance_to_target < self.alignment_start_distance {
        //     0.0
        // } else {
        //     1.0
        // }
        if distance_to_target < self.alignment_start_distance - self.ramp_width {
            0.0
        } else if distance_to_target < self.alignment_start_distance + self.ramp_width {
            (distance_to_target - (self.alignment_start_distance - self.ramp_width))
                / (2.0 * self.ramp_width)
        } else {
            1.0
        }
    }

    fn importance_derivative(&self, distance_to_target: f32) -> f32 {
        if distance_to_target < self.alignment_start_distance - self.ramp_width {
            0.0
        } else if distance_to_target < self.alignment_start_distance + self.ramp_width {
            1.0 / (2.0 * self.ramp_width)
        } else {
            0.0
        }
    }
}
