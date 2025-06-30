use nalgebra::RealField;
use num_dual::DualNum;

use coordinate_systems::Ground;
use linear_algebra::Orientation2;
use types::{
    motion_command::OrientationMode, parameters::StepPlanningOptimizationParameters,
    planned_path::Path, support_foot::Side, walk_volume_extents::WalkVolumeExtents,
};

use crate::{
    cost_fields::{
        path_distance::PathDistanceField, path_progress::PathProgressField,
        target_orientation::TargetOrientationField, walk_orientation::WalkOrientationField,
    },
    geometry::{
        angle::Angle,
        normalized_step::NormalizedStep,
        pose::{Pose, PoseGradient},
    },
    traits::{Length, PathProgress},
};

pub struct StepPlan<'a, T>(&'a [T]);

impl<'a, T> From<&'a [T]> for StepPlan<'a, T> {
    fn from(value: &'a [T]) -> Self {
        assert!(value.len() % 3 == 0);

        Self(value)
    }
}

impl<'a, T: RealField> StepPlan<'a, T> {
    pub fn steps(&self) -> impl Iterator<Item = NormalizedStep<T>> + 'a {
        self.0.chunks_exact(3).map(NormalizedStep::from_slice)
    }
}

#[derive(Clone, Debug)]
pub struct StepPlanning<'a> {
    pub path: &'a Path,
    pub target_orientation: Orientation2<Ground>,
    pub initial_pose: Pose<f32>,
    pub initial_support_foot: Side,
    pub orientation_mode: OrientationMode,
    pub parameters: &'a StepPlanningOptimizationParameters,
}

impl StepPlanning<'_> {
    pub fn step_end_poses<'a, T: RealField + DualNum<f32>>(
        &self,
        initial_pose: Pose<T>,
        initial_support_side: Side,
        walk_volume_extents: WalkVolumeExtents,
        step_plan: &StepPlan<'a, T>,
    ) -> impl Iterator<Item = Pose<T>> + 'a {
        step_plan.steps().scan(
            (initial_pose, initial_support_side),
            move |(pose, support_side), step| {
                *pose += step.unnormalize(&walk_volume_extents, *support_side);
                *support_side = support_side.opposite();

                Some(pose.clone())
            },
        )
    }

    pub fn cost(&self, pose: Pose<f32>) -> f32 {
        let cost_factors = &self.parameters.cost_factors;

        let progress = self.path.progress(pose.position);
        let path_length = self.path.length();

        let path_progress_cost =
            self.path_progress().cost(progress, path_length) * cost_factors.path_progress;
        let path_distance_cost =
            self.path_distance().cost(pose.position) * cost_factors.path_distance;
        let walk_orientation_cost =
            self.walk_orientation().cost(pose.clone()) * cost_factors.walk_orientation;
        let target_orientation_cost = self.target_orientation().cost(pose, progress, path_length)
            * cost_factors.target_orientation;

        path_progress_cost + path_distance_cost + walk_orientation_cost + target_orientation_cost
    }

    pub fn grad(&self, pose: Pose<f32>) -> PoseGradient<f32> {
        let cost_factors = &self.parameters.cost_factors;

        let progress = self.path.progress(pose.position);
        let forward = self.path.forward(pose.position);
        let path_length = self.path.length();

        let path_progress_gradient =
            self.path_progress().grad(progress, forward, path_length) * cost_factors.path_progress;
        let path_distance_gradient =
            self.path_distance().grad(pose.position) * cost_factors.path_distance;
        let walk_orientation_gradient =
            self.walk_orientation().grad(pose.clone()) * cost_factors.walk_orientation;
        let target_orientation_gradient =
            self.target_orientation().grad(pose, progress, path_length)
                * cost_factors.target_orientation;

        PoseGradient {
            position: path_progress_gradient + path_distance_gradient,
            orientation: 0.0,
        } + walk_orientation_gradient
            + target_orientation_gradient
    }

    fn path_distance(&self) -> PathDistanceField<'_> {
        PathDistanceField { path: self.path }
    }

    fn path_progress(&self) -> PathProgressField {
        PathProgressField {
            smoothness: self.parameters.path_progress_smoothness,
        }
    }

    fn walk_orientation(&self) -> WalkOrientationField {
        WalkOrientationField {
            orientation_mode: self.orientation_mode,
        }
    }

    fn target_orientation(&self) -> TargetOrientationField {
        TargetOrientationField {
            target_orientation: Angle(self.target_orientation.angle()),
            alignment_start_distance: self.parameters.alignment_start_distance,
            ramp_width: self.parameters.alignment_start_smoothness,
        }
    }
}
