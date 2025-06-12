use nalgebra::{RealField, Scalar};
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
    pub path: Path<'a>,
    pub target_orientation: Orientation2<Ground>,
    pub initial_pose: Pose<f32>,
    pub initial_support_foot: Side,
    pub orientation_mode: OrientationMode,
    pub parameters: &'a StepPlanningOptimizationParameters,
}

impl StepPlanning<'_> {
    pub fn planned_steps<'a, T: RealField + DualNum<f32>>(
        &self,
        initial_pose: Pose<T>,
        initial_support_side: Side,
        walk_volume_extents: WalkVolumeExtents,
        step_plan: &StepPlan<'a, T>,
    ) -> impl Iterator<Item = PlannedStep<T>> + 'a {
        step_plan.steps().scan(
            (initial_pose, initial_support_side),
            move |(pose, support_side), step| {
                *pose += step.unnormalize(&walk_volume_extents, *support_side);

                let planned_step = PlannedStep {
                    pose: pose.clone(),
                    step,
                };

                *support_side = support_side.opposite();

                Some(planned_step)
            },
        )
    }

    pub fn cost(&self, planned_step: PlannedStep<f32>) -> f32 {
        let StepPlanningOptimizationParameters {
            path_progress_reward,
            path_distance_penalty,
            target_orientation_penalty,
            walk_orientation_penalty,
            ..
        } = *self.parameters;
        let PlannedStep { pose, .. } = planned_step;

        let path_progress_cost = self.path_progress().cost(pose.position) * path_progress_reward;
        let path_distance_cost = self.path_distance().cost(pose.position) * path_distance_penalty;
        let walk_orientation_cost =
            self.walk_orientation().cost(pose.clone()) * walk_orientation_penalty;
        let target_orientation_cost =
            self.target_orientation().cost(pose) * target_orientation_penalty;

        path_progress_cost + path_distance_cost + walk_orientation_cost + target_orientation_cost
    }

    pub fn grad(&self, planned_step: PlannedStep<f32>) -> PlannedStepGradient<f32> {
        let StepPlanningOptimizationParameters {
            path_progress_reward,
            path_distance_penalty,
            target_orientation_penalty,
            walk_orientation_penalty,
            ..
        } = *self.parameters;
        let PlannedStep { pose, .. } = planned_step;

        let path_progress_gradient =
            self.path_progress().grad(pose.position) * path_progress_reward;
        let path_distance_gradient =
            self.path_distance().grad(pose.position) * path_distance_penalty;
        let walk_orientation_gradient =
            self.walk_orientation().grad(pose.clone()) * walk_orientation_penalty;
        let target_orientation_gradient =
            self.target_orientation().grad(pose) * target_orientation_penalty;

        PlannedStepGradient {
            pose: walk_orientation_gradient
                + target_orientation_gradient
                + PoseGradient {
                    position: path_distance_gradient + path_progress_gradient,
                    orientation: 0.0,
                },
            step: NormalizedStep {
                // TODO
                forward: 0.0,
                left: 0.0,
                turn: 0.0,
            },
        }
    }

    fn path_distance(&self) -> PathDistanceField<'_> {
        PathDistanceField { path: self.path }
    }

    fn path_progress(&self) -> PathProgressField<'_> {
        PathProgressField {
            path: self.path,
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
            path: self.path,
            alignment_start_distance: self.parameters.alignment_start_distance,
            ramp_width: self.parameters.alignment_start_smoothness,
        }
    }
}

#[derive(Debug)]
pub struct PlannedStep<T: Scalar> {
    /// Pose reached after this step
    pub pose: Pose<T>,
    pub step: NormalizedStep<T>,
}

pub struct PlannedStepGradient<T: Scalar> {
    pub pose: PoseGradient<T>,
    pub step: NormalizedStep<T>,
}
