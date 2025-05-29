use nalgebra::Scalar;

use types::step::Step;

use crate::{
    geometry::Pose,
    loss_fields::{
        path_distance::PathDistanceField, path_progress::PathProgressField,
        step_size::StepSizeField, target_orientation::TargetOrientationField,
        walk_orientation::WalkOrientationField,
    },
    step_plan::PlannedStep,
};

pub struct StepPlanningLossField<'a> {
    pub path_distance_field: PathDistanceField<'a>,
    pub path_distance_penalty: f32,
    pub path_progress_field: PathProgressField<'a>,
    pub path_progress_reward: f32,
    pub step_size_field: StepSizeField,
    pub step_size_penalty: f32,
    pub target_orientation_field: TargetOrientationField<'a>,
    pub target_orientation_penalty: f32,
    pub walk_orientation_field: WalkOrientationField,
    pub walk_orientation_penalty: f32,
}

pub struct PlannedStepGradient<T: Scalar> {
    pub pose: Pose<T>,
    pub step: Step<T>,
}

impl StepPlanningLossField<'_> {
    pub fn loss(&self, parameter: PlannedStep<f32>) -> f32 {
        let PlannedStep { pose, step } = parameter;

        let distance_loss = self.path_distance_field.loss(pose.position);
        let progress_loss = self.path_progress_field.loss(pose.position);
        let step_size_loss = self.step_size_field.loss(step);
        let target_orientation_loss = self.target_orientation_field.loss(pose.clone());
        let walk_orientation_loss = self.walk_orientation_field.loss(pose);

        distance_loss * self.path_distance_penalty
            + progress_loss * self.path_progress_reward
            + step_size_loss * self.step_size_penalty
            + target_orientation_loss * self.target_orientation_penalty
            + walk_orientation_loss * self.walk_orientation_penalty
    }

    pub fn grad(&self, parameter: PlannedStep<f32>) -> PlannedStepGradient<f32> {
        let PlannedStep { pose, step } = parameter;

        let distance_loss_gradient =
            self.path_distance_field.grad(pose.position) * self.path_distance_penalty;
        let progress_loss_gradient =
            self.path_progress_field.grad(pose.position) * self.path_progress_reward;
        let step_size_loss = self.step_size_field.grad(step) * self.step_size_penalty;
        let target_orientation_gradient =
            self.target_orientation_field.grad(pose.clone()) * self.target_orientation_penalty;
        let walk_orientation_gradient =
            self.walk_orientation_field.grad(pose) * self.walk_orientation_penalty;

        // dbg!(
        //     &distance_loss_gradient,
        //     &progress_loss_gradient,
        //     &step_size_loss,
        //     &target_orientation_gradient,
        //     &walk_orientation_gradient
        // );

        PlannedStepGradient {
            pose: walk_orientation_gradient
                + target_orientation_gradient
                + Pose {
                    position: (distance_loss_gradient + progress_loss_gradient).as_point(),
                    orientation: 0.0,
                },
            step: step_size_loss,
        }
    }
}
