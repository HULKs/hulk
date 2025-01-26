use coordinate_systems::Ground;
use linear_algebra::Orientation2;
use nalgebra::{RealField, Scalar};

use types::{
    motion_command::OrientationMode,
    planned_path::Path,
    step::{Step, StepAndSupportFoot},
    support_foot::Side,
};

use crate::{
    geometry::{angle::Angle, pose::PoseAndSupportFoot, Pose},
    loss_fields::{
        path_distance::PathDistanceField,
        path_progress::PathProgressField,
        step_planning::StepPlanningLossField,
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

#[derive(Clone, Debug)]
pub struct StepPlanning {
    pub path: Path,
    pub target_orientation: Orientation2<Ground>,
    pub alignment_start_distance: f32,
    pub alignment_start_smoothness: f32,
    pub initial_pose: Pose<f32>,
    pub initial_support_foot: Side,
    pub path_progress_smoothness: f32,
    pub path_progress_reward: f32,
    pub path_distance_penalty: f32,
    pub step_size_penalty: f32,
    pub walk_volume_coefficients: WalkVolumeCoefficients,
    pub orientation_mode: OrientationMode,
    pub target_orientation_penalty: f32,
    pub walk_orientation_penalty: f32,
    pub num_steps: usize,
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

    pub fn loss_field(&self) -> StepPlanningLossField {
        StepPlanningLossField {
            path_distance_field: PathDistanceField { path: &self.path },
            path_distance_penalty: self.path_distance_penalty,
            path_progress_field: PathProgressField {
                path: &self.path,
                smoothness: self.path_progress_smoothness,
            },
            path_progress_reward: self.path_progress_reward,
            step_size_field: StepSizeField {
                walk_volume_coefficients: self.walk_volume_coefficients.clone(),
            },
            step_size_penalty: self.step_size_penalty,
            target_orientation_field: TargetOrientationField {
                target_orientation: Angle(self.target_orientation.angle()),
                path: &self.path,
                alignment_start_distance: self.alignment_start_distance,
                ramp_width: self.alignment_start_smoothness,
            },
            target_orientation_penalty: self.target_orientation_penalty,
            walk_orientation_field: WalkOrientationField {
                orientation_mode: self.orientation_mode,
            },
            walk_orientation_penalty: self.walk_orientation_penalty,
        }
    }
}

#[derive(Debug)]
pub struct PlannedStep<T: Scalar> {
    /// Pose reached after this step
    pub pose: Pose<T>,
    pub step: StepAndSupportFoot<T>,
}

#[cfg(test)]
mod tests {
    use linear_algebra::{point, Point2};

    use crate::{geometry::Pose, step_plan::Step};

    #[test]
    fn test_pose_step_addition() {
        let pose = Pose {
            position: Point2::origin(),
            orientation: 0.0,
        };
        let step = Step {
            forward: 2.0,
            left: 1.0,
            turn: 3.0,
        };
        let new_pose = pose + step;
        assert_eq!(
            new_pose,
            Pose {
                position: point![2.0, 1.0],
                orientation: 3.0
            }
        );
    }
}
