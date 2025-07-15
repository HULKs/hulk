use color_eyre::Result;
use nalgebra::{DVector, Dyn, U1};
use num_dual::{Derivative, DualNum, DualNumFloat, DualVec};
use optimization_engine::{
    constraints::Constraint,
    panoc::{PANOCCache, PANOCOptimizer},
    Optimizer, Problem, SolverError,
};

use coordinate_systems::Ground;
use linear_algebra::Orientation2;
use step_planning::{
    geometry::pose::Pose,
    step_plan::StepPlan,
    traits::{ScaledGradient, UnwrapDual, WrapDual},
    StepPlanning, VARIABLES_PER_STEP,
};
use types::{
    motion_command::OrientationMode, parameters::StepPlanningOptimizationParameters,
    planned_path::Path, support_foot::Side, walk_volume_extents::WalkVolumeExtents,
};

struct WalkVolumeConstraint;

impl Constraint for WalkVolumeConstraint {
    fn project(&self, variables: &mut [f64]) {
        debug_assert!(variables.len() % VARIABLES_PER_STEP == 0);

        for step in variables.chunks_exact_mut(3) {
            let squared_magnitude = step.iter().map(|x| x.powi(2)).sum::<f64>();

            if squared_magnitude > 1.0 {
                let factor = squared_magnitude.sqrt().recip();
                step.iter_mut().for_each(|x| *x *= factor);
            }
        }
    }

    fn is_convex(&self) -> bool {
        true
    }
}

fn duals<F: DualNumFloat + DualNum<F>>(reals: &[F]) -> Vec<DualVec<F, F, Dyn>> {
    let num_variables = reals.len();

    reals
        .iter()
        .enumerate()
        .map(|(row, real)| {
            DualVec::new(
                *real,
                Derivative::some(DVector::from_fn(num_variables, |i, _| {
                    if i == row {
                        F::one()
                    } else {
                        F::zero()
                    }
                })),
            )
        })
        .collect()
}

fn cost(variables: &[f32], step_planning: &StepPlanning) -> f32 {
    let step_plan = StepPlan::from(variables);

    let cost = step_planning
        .step_end_poses(
            step_planning.initial_pose.clone(),
            step_planning.initial_support_foot,
            step_planning.walk_volume_extents.clone(),
            &step_plan,
        )
        .map(|planned_step| step_planning.cost(planned_step))
        .sum();

    cost
}

fn open_cost(
    step_planning: &StepPlanning,
    variables: &[f64],
    out_cost: &mut f64,
) -> Result<(), SolverError> {
    let variables: Vec<f32> = variables.iter().map(|&x| x as f32).collect();

    let cost = cost(&variables, step_planning);

    *out_cost = cost as f64;

    Ok(())
}

fn gradient(variables: &[f32], step_planning: &StepPlanning) -> DVector<f32> {
    let num_variables = variables.len();
    let dual_variables = duals(variables);

    let step_plan = StepPlan::from(dual_variables.as_slice());

    let gradient = step_planning
        .step_end_poses(
            step_planning.initial_pose.clone().wrap_dual(),
            step_planning.initial_support_foot,
            step_planning.walk_volume_extents.clone(),
            &step_plan,
        )
        .map(|dual_planned_step| {
            let (planned_step, planned_step_gradients) = dual_planned_step.unwrap_dual();

            let derivatives = step_planning.grad(planned_step);

            planned_step_gradients
                .scaled_gradient(derivatives)
                .unwrap_generic(Dyn(num_variables), U1)
        })
        .sum::<DVector<f32>>();

    normalize_gradient(gradient, 2.0)
}

fn open_gradient(
    step_planning: &StepPlanning,
    variables: &[f64],
    out_gradient: &mut [f64],
) -> Result<(), SolverError> {
    let variables: Vec<f32> = variables.iter().map(|&x| x as f32).collect();

    let gradient = gradient(&variables, step_planning);

    debug_assert!(out_gradient.len() == gradient.len());

    for (&src, out) in gradient.iter().zip(out_gradient) {
        *out = src as f64;
    }

    Ok(())
}

#[expect(clippy::too_many_arguments)]
pub fn plan_steps(
    path: &Path,
    orientation_mode: OrientationMode,
    target_orientation: Orientation2<Ground>,
    distance_to_be_aligned: f32,
    initial_pose: Pose<f32>,
    initial_support_foot: Side,
    variables: &mut [f64],
    walk_volume_extents: &WalkVolumeExtents,
    parameters: &StepPlanningOptimizationParameters,
) -> Result<(DVector<f32>, f32)> {
    let step_planning = StepPlanning {
        path,
        initial_pose: initial_pose.clone(),
        initial_support_foot,
        parameters,
        orientation_mode,
        target_orientation,
        distance_to_be_aligned,
        walk_volume_extents,
    };

    let problem = Problem::new(
        &WalkVolumeConstraint,
        |variables, out_gradient| open_gradient(&step_planning, variables, out_gradient),
        |variables, out_cost| open_cost(&step_planning, variables, out_cost),
    );

    let n = parameters.num_steps * VARIABLES_PER_STEP;
    let lbfgs_memory = 10;
    let tolerance = 1e-6;
    let mut panoc_cache = PANOCCache::new(n, tolerance, lbfgs_memory).with_cbfgs_parameters(
        // These parameters are needed to fix occasional instability.
        // This would probably not be necessary if we wouldn't be casting between f32 and f64
        // in the solver interface.
        // TODO(rmburg): Either use f32 in the solver or f64 in step planning
        1.0,  // default
        1e-8, // default
        1e-6, // reduced from 1e-10
    );

    let mut panoc =
        PANOCOptimizer::new(problem, &mut panoc_cache).with_max_iter(parameters.optimizer_steps);

    let cost = match panoc.solve(variables) {
        Ok(status) => status.cost_value() as f32,
        Err(e) => {
            eprint!("PANOC error: {e:?}");
            -1.0
        }
    };

    let variables = variables.iter().map(|&x| x as f32).collect::<Vec<_>>();

    // TODO(rmburg) remove/refactor
    let gradient = gradient(&variables, &step_planning);

    Ok((gradient, cost))
}

fn normalize_gradient(mut gradient: DVector<f32>, max_squared_magnitude: f32) -> DVector<f32> {
    for chunk in gradient.as_mut_slice().chunks_exact_mut(3) {
        let squared_magnitude = chunk.iter().map(|x| x.powi(2)).sum::<f32>();

        if squared_magnitude > max_squared_magnitude {
            let factor = squared_magnitude.sqrt().recip();
            for variable in chunk.iter_mut() {
                *variable *= factor;
            }
        }
    }

    gradient
}
