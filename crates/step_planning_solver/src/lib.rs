use color_eyre::Result;
use levenberg_marquardt::LeastSquaresProblem;
use nalgebra::{vector, DVector, Dyn, Owned, U1};
use num_dual::{Derivative, DualNum, DualNumFloat, DualVec};

use coordinate_systems::Ground;
use linear_algebra::Orientation2;
use step_planning::{
    geometry::Pose,
    step_plan::{StepPlan, StepPlanning},
    traits::{ScaledGradient, UnwrapDual, WrapDual},
};
use types::{
    motion_command::OrientationMode, parameters::StepPlanningOptimizationParameters,
    planned_path::Path, support_foot::Side,
};

fn duals<F: DualNumFloat + DualNum<F>>(reals: &DVector<F>) -> DVector<DualVec<F, F, Dyn>> {
    let num_variables = reals.nrows();

    reals.map_with_location(|row, _, real| {
        DualVec::new(
            real,
            Derivative::some(DVector::from_fn(num_variables, |i, _| {
                if i == row {
                    F::one()
                } else {
                    F::zero()
                }
            })),
        )
    })
}

#[derive(Clone, Debug)]
struct StepPlanningProblem {
    step_planning: StepPlanning,
    variables: DVector<f32>,
}

impl LeastSquaresProblem<f32, U1, Dyn> for StepPlanningProblem {
    type ResidualStorage = Owned<f32, U1, U1>;
    type JacobianStorage = Owned<f32, U1, Dyn>;
    type ParameterStorage = Owned<f32, Dyn, U1>;

    fn set_params(&mut self, x: &nalgebra::Vector<f32, Dyn, Self::ParameterStorage>) {
        self.variables = x.clone();
    }

    fn params(&self) -> nalgebra::Vector<f32, Dyn, Self::ParameterStorage> {
        self.variables.clone()
    }

    fn residuals(&self) -> Option<nalgebra::Vector<f32, U1, Self::ResidualStorage>> {
        let step_plan = StepPlan::from(self.variables.as_slice());

        let loss = self
            .step_planning
            .planned_steps(
                self.step_planning
                    .initial_pose
                    .clone()
                    .with_support_foot(self.step_planning.initial_support_foot),
                &step_plan,
            )
            .map(|planned_step| self.step_planning.cost(planned_step))
            .sum();

        // eprintln!("loss: {loss}\n\t({:.4?})", self.variables.as_slice());

        Some(vector![loss])
    }

    fn jacobian(&self) -> Option<nalgebra::Matrix<f32, U1, Dyn, Self::JacobianStorage>> {
        let num_variables = self.variables.nrows();
        let dual_param = duals(&self.variables);

        let step_plan = StepPlan::from(dual_param.as_slice());

        let gradient: DVector<f32> = self
            .step_planning
            .planned_steps(
                self.step_planning
                    .initial_pose
                    .clone()
                    .with_support_foot(self.step_planning.initial_support_foot)
                    .wrap_dual(),
                &step_plan,
            )
            .map(|dual_planned_step| {
                let (planned_step, planned_step_gradients) = dual_planned_step.unwrap_dual();

                let derivatives = self.step_planning.grad(planned_step);

                planned_step_gradients
                    .scaled_gradient(derivatives)
                    .unwrap_generic(Dyn(num_variables), U1)
            })
            .sum();

        // let step_planning_loss = self.step_planning.loss_field();

        // let step_plan = StepPlan::from(self.variables.as_slice());

        // let loss: f32 = self
        //     .step_planning
        //     .planned_steps(
        //         self.step_planning
        //             .initial_pose
        //             .clone()
        //             .with_support_foot(self.step_planning.initial_support_foot),
        //         &step_plan,
        //     )
        //     .map(|planned_step| step_planning_loss.loss(planned_step))
        //     .sum();

        // eprintln!(
        //     "grad: {loss}\n\t({:.4?})\n\t[{:.4?}]",
        //     self.variables.as_slice(),
        //     &gradient.as_slice()
        // );

        Some(gradient.transpose())
    }
}

pub fn plan_steps(
    path: Path,
    orientation_mode: OrientationMode,
    target_orientation: Orientation2<Ground>,
    initial_pose: Pose<f32>,
    initial_support_foot: Side,
    initial_parameter_guess: DVector<f32>,
    parameters: &StepPlanningOptimizationParameters,
) -> Result<(DVector<f32>, DVector<f32>)> {
    let mut problem = StepPlanningProblem {
        step_planning: StepPlanning {
            path,
            initial_pose: initial_pose.clone(),
            initial_support_foot,
            parameters: parameters.clone(),
            orientation_mode,
            target_orientation,
        },
        variables: initial_parameter_guess,
    };

    gradient_decent(&mut problem, parameters.optimizer_steps);

    // TODO remove/refactor
    let gradient = problem.jacobian().unwrap().transpose();

    Ok((problem.variables, gradient))
}

fn gradient_decent(problem: &mut StepPlanningProblem, optimizer_steps: usize) {
    for _ in 0..optimizer_steps {
        let gradient = problem.jacobian().unwrap().transpose().cap_magnitude(1.0);

        if gradient[0].is_nan() {
            dbg!(problem, gradient);
            panic!();
        }
        problem.variables -= gradient * 0.003;
    }
}

// #[cfg(test)]
// mod tests {
//     use geometry::line_segment::LineSegment;
//     use linear_algebra::point;
//     use types::planned_path::PathSegment;

//     use super::*;

//     #[test]
//     fn foo() {
//         let path = Path {
//             segments: vec![PathSegment::LineSegment(LineSegment(
//                 point![0.0, 0.0,],
//                 point![1.5393465, 1.0662808,],
//             ))],
//         };

//         let initial_parameter_guess = DVector::zeros(15);
//         let initial_pose = Pose {
//             position: point![-0.0014890115, 0.0011079945,],
//             orientation: 0.013183115,
//         };
//         let initial_support_foot = Side::Left;

//         let line_search = MoreThuenteLineSearch::new()
//             .with_bounds(f32::EPSILON.sqrt(), 1.0)
//             .unwrap();
//         let solver = LBFGS::new(line_search, 10);

//         let problem = StepPlanningProblem {
//             step_planning: StepPlanning {
//                 path: path.clone(),
//                 initial_pose: initial_pose.clone(),
//                 initial_support_foot,
//                 path_progress_reward: 5.0,
//                 path_distance_penalty: 50.0,
//                 path_progress_smoothness: 1.0,
//                 step_size_penalty: 1.0,
//                 walk_volume_coefficients: WalkVolumeCoefficients::from_extents_and_exponents(
//                     &WalkVolumeExtents {
//                         forward: 0.045,
//                         backward: 0.04,
//                         outward: 0.1,
//                         inward: 0.01,
//                         outward_rotation: 1.0,
//                         inward_rotation: 1.0,
//                     },
//                     1.5,
//                     2.0,
//                 ),
//             },
//         };

//         let result = Executor::new(problem.clone(), solver)
//             .configure(|state| state.param(initial_parameter_guess).max_iters(1000))
//             .run()
//             .map_err(|error| eyre!("Executor failed: {error:?}"))
//             .unwrap();

//         if let TerminationStatus::Terminated(SolverExit(reason)) = result.state.termination_status {
//             println!("executor failed: {reason:?}");
//             // dbg!(path.segments, initial_pose, initial_support_foot);

//             panic!();
//         };
//     }
// }

#[cfg(test)]
mod tests {
    use geometry::line_segment::LineSegment;
    use levenberg_marquardt::LeastSquaresProblem;
    use linear_algebra::{point, Orientation2};
    use nalgebra::DVector;

    use step_planning::{geometry::Pose, step_plan::StepPlanning};
    use types::{
        motion_command::OrientationMode,
        parameters::StepPlanningOptimizationParameters,
        planned_path::{Path, PathSegment},
        support_foot::Side,
        walk_volume_extents::WalkVolumeExtents,
    };

    use crate::StepPlanningProblem;

    #[test]
    fn foo() {
        let problem = StepPlanningProblem {
            step_planning: StepPlanning {
                path: Path {
                    segments: vec![PathSegment::LineSegment(LineSegment(
                        point![0.0, 0.0,],
                        point![0.0, 1.2924697e-26,],
                    ))],
                },
                target_orientation: Orientation2::from_cos_sin_unchecked(1.0, -8.893846e-21),
                parameters: StepPlanningOptimizationParameters {
                    alignment_start_distance: 0.1,
                    alignment_start_smoothness: 0.05,
                    path_progress_smoothness: 0.05,
                    path_progress_reward: 5.0,
                    path_distance_penalty: 50.0,
                    step_size_penalty: 0.5,
                    walk_volume_extents: WalkVolumeExtents {
                        forward: 0.045,
                        backward: 0.04,
                        outward: 0.1,
                        inward: 0.01,
                        outward_rotation: 1.0,
                        inward_rotation: 1.0,
                    },
                    target_orientation_penalty: 1.0,
                    walk_orientation_penalty: 1.0,
                    num_steps: 15,
                    optimizer_steps: 50,
                },
                initial_pose: Pose {
                    position: point![-0.0, 0.0,],
                    orientation: -0.0,
                },
                initial_support_foot: Side::Right,
                orientation_mode: OrientationMode::LookAt(point![0.99999857, -8.893833e-21,]),
            },
            variables: DVector::zeros(15),
        };

        let grad = problem.jacobian().unwrap();

        if grad.into_iter().any(|x| x.is_nan()) {
            dbg!(grad);
            panic!();
        }
    }
}
