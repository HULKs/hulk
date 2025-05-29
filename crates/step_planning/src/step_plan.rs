use nalgebra::{RealField, Scalar};

use coordinate_systems::Ground;
use linear_algebra::Orientation2;
use types::{
    motion_command::OrientationMode,
    parameters::StepPlanningOptimizationParameters,
    planned_path::Path,
    step::{Step, StepAndSupportFoot},
    support_foot::Side,
};

use crate::{
    geometry::{angle::Angle, pose::PoseAndSupportFoot, Pose},
    loss_fields::{
        path_distance::PathDistanceField,
        path_progress::PathProgressField,
        step_size::{StepSizeField, WalkVolumeCoefficients},
        target_orientation::TargetOrientationField,
        walk_orientation::WalkOrientationField,
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
    pub fn steps(&self) -> impl Iterator<Item = Step<T>> + 'a {
        self.0.chunks_exact(3).map(Step::from_slice)
    }
}

// TODO borrow parameters, path, initial_pose,...
#[derive(Clone, Debug)]
pub struct StepPlanning {
    pub path: Path,
    pub target_orientation: Orientation2<Ground>,
    pub initial_pose: Pose<f32>,
    pub initial_support_foot: Side,
    pub orientation_mode: OrientationMode,
    pub parameters: StepPlanningOptimizationParameters,
}

impl StepPlanning {
    pub fn planned_steps<'a, T: RealField>(
        &self,
        initial_pose: PoseAndSupportFoot<T>,
        step_plan: &StepPlan<'a, T>,
    ) -> impl Iterator<Item = PlannedStep<T>> + 'a {
        step_plan.steps().scan(initial_pose, |pose, step| {
            pose.pose += step.clone();

            let planned_step = PlannedStep {
                pose: pose.pose.clone(),
                step: {
                    StepAndSupportFoot {
                        step,
                        support_foot: pose.support_foot,
                    }
                },
            };

            pose.support_foot = pose.support_foot.opposite();

            Some(planned_step)
        })
    }

    pub fn cost(&self, planned_step: PlannedStep<f32>) -> f32 {
        let StepPlanningOptimizationParameters {
            path_progress_reward,
            path_distance_penalty,
            step_size_penalty,
            target_orientation_penalty,
            walk_orientation_penalty,
            ..
        } = self.parameters;
        let PlannedStep { pose, step } = planned_step;

        let path_progress_cost = self.path_progress().loss(pose.position) * path_progress_reward;
        let path_distance_cost = self.path_distance().loss(pose.position) * path_distance_penalty;
        let walk_orientation_cost =
            self.walk_orientation().loss(pose.clone()) * walk_orientation_penalty;
        let target_orientation_cost =
            self.target_orientation().loss(pose) * target_orientation_penalty;
        let step_size_cost = self.step_size().loss(step) * step_size_penalty;

        path_progress_cost
            + path_distance_cost
            + walk_orientation_cost
            + target_orientation_cost
            + step_size_cost
    }

    pub fn grad(&self, planned_step: PlannedStep<f32>) -> PlannedStepGradient<f32> {
        let StepPlanningOptimizationParameters {
            path_progress_reward,
            path_distance_penalty,
            step_size_penalty,
            target_orientation_penalty,
            walk_orientation_penalty,
            ..
        } = self.parameters;
        let PlannedStep { pose, step } = planned_step;

        let path_progress_gradient =
            self.path_progress().grad(pose.position) * path_progress_reward;
        let path_distance_gradient =
            self.path_distance().grad(pose.position) * path_distance_penalty;
        let walk_orientation_gradient =
            self.walk_orientation().grad(pose.clone()) * walk_orientation_penalty;
        let target_orientation_gradient =
            self.target_orientation().grad(pose) * target_orientation_penalty;
        let step_size_gradient = self.step_size().grad(step) * step_size_penalty;

        PlannedStepGradient {
            pose: walk_orientation_gradient
                + target_orientation_gradient
                + Pose {
                    position: (path_distance_gradient + path_progress_gradient).as_point(),
                    orientation: 0.0,
                },
            step: step_size_gradient,
        }
    }

    fn path_distance(&self) -> PathDistanceField<'_> {
        PathDistanceField { path: &self.path }
    }

    fn path_progress(&self) -> PathProgressField<'_> {
        PathProgressField {
            path: &self.path,
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
            path: &self.path,
            alignment_start_distance: self.parameters.alignment_start_distance,
            ramp_width: self.parameters.alignment_start_smoothness,
        }
    }

    fn step_size(&self) -> StepSizeField {
        StepSizeField {
            walk_volume_coefficients: WalkVolumeCoefficients::from_extents(
                &self.parameters.walk_volume_extents,
            ),
        }
    }
}

#[derive(Debug)]
pub struct PlannedStep<T: Scalar> {
    /// Pose reached after this step
    pub pose: Pose<T>,
    pub step: StepAndSupportFoot<T>,
}

pub struct PlannedStepGradient<T: Scalar> {
    pub pose: Pose<T>,
    pub step: Step<T>,
}
