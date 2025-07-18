use std::{array, f32::consts::PI};

use color_eyre::Result;
use geometry::direction::Direction;
use nalgebra::{Const, SVector, U1};
use num_dual::{Derivative, DualNum, DualNumFloat, DualVec};
use optimization_engine::{
    constraints::Constraint,
    panoc::{PANOCCache, PANOCOptimizer},
    Optimizer, Problem, SolverError,
};

use coordinate_systems::Ground;
use linear_algebra::Orientation2;
use step_planning::{
    geometry::{angle::Angle, orientation::Orientation, pose::Pose},
    step_plan::StepPlan,
    traits::{ForwardAtEndPoint, ScaledGradient, WrapDual},
    StepPlanning, TargetOrientationPathSide, NUM_VARIABLES, VARIABLES_PER_STEP,
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

fn duals<F: DualNumFloat + DualNum<F>>(
    reals: &[F],
) -> [DualVec<F, F, Const<NUM_VARIABLES>>; NUM_VARIABLES] {
    debug_assert_eq!(reals.len(), NUM_VARIABLES);

    array::from_fn(|row| {
        let real = reals[row];
        DualVec::new(
            real,
            Derivative::some(SVector::from_fn(|i, _| {
                if i == row {
                    F::one()
                } else {
                    F::zero()
                }
            })),
        )
    })
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
    debug_assert_eq!(variables.len(), NUM_VARIABLES);
    let variables: [f32; NUM_VARIABLES] = array::from_fn(|i| variables[i] as f32);

    let cost = cost(&variables, step_planning);

    *out_cost = cost as f64;

    Ok(())
}

fn gradient(
    variables: &[f32; NUM_VARIABLES],
    step_planning: &StepPlanning,
) -> SVector<f32, NUM_VARIABLES> {
    let dual_variables = duals(variables);

    let step_plan = StepPlan::from(dual_variables.as_slice());

    let gradient = step_planning
        .step_end_poses(
            WrapDual::wrap_dual(step_planning.initial_pose.clone()),
            step_planning.initial_support_foot,
            step_planning.walk_volume_extents.clone(),
            &step_plan,
        )
        .map(|dual_planned_step| {
            let (planned_step, planned_step_gradients) = dual_planned_step.unwrap_dual();

            let derivatives = step_planning.grad(planned_step);

            planned_step_gradients
                .scaled_gradient(derivatives)
                .unwrap_generic(Const::<NUM_VARIABLES>, U1)
        })
        .sum::<SVector<f32, NUM_VARIABLES>>();

    normalize_gradient(gradient, 2.0)
}

fn open_gradient(
    step_planning: &StepPlanning,
    variables: &[f64],
    out_gradient: &mut [f64],
) -> Result<(), SolverError> {
    debug_assert_eq!(variables.len(), NUM_VARIABLES);
    let variables: [f32; NUM_VARIABLES] = array::from_fn(|i| variables[i] as f32);

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
    variables: &mut [f64; NUM_VARIABLES],
    walk_volume_extents: &WalkVolumeExtents,
    parameters: &StepPlanningOptimizationParameters,
) -> Result<(SVector<f32, NUM_VARIABLES>, f32)> {
    let target_orientation = Orientation(target_orientation.angle());
    let target_orientation_path_side = calculate_target_orientation_path_side(
        path,
        target_orientation,
        parameters.target_orientation_ahead_tolerance,
    );

    let step_planning = StepPlanning {
        path,
        initial_pose: initial_pose.clone(),
        initial_support_foot,
        parameters,
        orientation_mode,
        target_orientation,
        target_orientation_path_side,
        distance_to_be_aligned,
        walk_volume_extents,
    };

    let problem = Problem::new(
        &WalkVolumeConstraint,
        |variables, out_gradient| open_gradient(&step_planning, variables, out_gradient),
        |variables, out_cost| open_cost(&step_planning, variables, out_cost),
    );

    let lbfgs_memory = 10;
    let tolerance = 1e-6;
    let mut panoc_cache = PANOCCache::new(NUM_VARIABLES, tolerance, lbfgs_memory)
        .with_cbfgs_parameters(
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

    let variables = array::from_fn(|i| variables[i] as f32);

    // TODO(rmburg) remove/refactor
    let gradient = gradient(&variables, &step_planning);

    Ok((gradient, cost))
}

fn calculate_target_orientation_path_side(
    path: &Path,
    target_orientation: Orientation<f32>,
    ahead_tolerance: f32,
) -> TargetOrientationPathSide {
    let forward_at_end_of_path = path.forward_at_end_point();
    let target_forward_to_target_orientation =
        forward_at_end_of_path.angle_to(target_orientation, Direction::Counterclockwise);

    if target_forward_to_target_orientation.absolute_difference(Angle(0.0)) <= ahead_tolerance {
        TargetOrientationPathSide::RoughlyAhead
    } else if target_forward_to_target_orientation.0 > PI {
        TargetOrientationPathSide::Right
    } else {
        TargetOrientationPathSide::Left
    }
}

fn normalize_gradient(
    mut gradient: SVector<f32, NUM_VARIABLES>,
    max_squared_magnitude: f32,
) -> SVector<f32, NUM_VARIABLES> {
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

#[cfg(test)]
mod tests {
    use std::f32::consts::{FRAC_PI_2, PI};

    use step_planning::{geometry::orientation::Orientation, test_path, TargetOrientationPathSide};

    use crate::calculate_target_orientation_path_side;

    #[test]
    fn target_orientation_path_side() {
        let path = test_path();

        assert_eq!(
            calculate_target_orientation_path_side(&path, Orientation(FRAC_PI_2), 0.5),
            TargetOrientationPathSide::RoughlyAhead
        );

        assert_eq!(
            calculate_target_orientation_path_side(&path, Orientation(PI), 0.5),
            TargetOrientationPathSide::Left
        );

        assert_eq!(
            calculate_target_orientation_path_side(&path, Orientation(0.0), 0.5),
            TargetOrientationPathSide::Right
        );
    }
}
