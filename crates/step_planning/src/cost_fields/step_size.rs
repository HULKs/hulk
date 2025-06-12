use types::walk_volume_extents::WalkVolumeExtents;

use crate::geometry::normalized_step::NormalizedStep;

pub struct StepSizeField {
    pub walk_volume_extents: WalkVolumeExtents,
}

impl StepSizeField {
    pub fn cost(&self, step: NormalizedStep<f32>) -> f32 {
        let squared_magnitude = step.forward.powi(2) + step.left.powi(2) + step.turn.powi(2);

        penalty_function(squared_magnitude)
    }

    pub fn grad(&self, step: NormalizedStep<f32>) -> NormalizedStep<f32> {
        let squared_magnitude = step.forward.powi(2) + step.left.powi(2) + step.turn.powi(2);
        let squared_magnitude_gradient = NormalizedStep {
            forward: step.forward * 2.0,
            left: step.left * 2.0,
            turn: step.turn * 2.0,
        };

        squared_magnitude_gradient * penalty_function_derivative(squared_magnitude)
    }
}

// To reproduce the walk volume function and its gradient:
//
// ```python
// from sympy import symbols, init_printing
// init_printing(use_unicode=True)
//
// f, l, a, cf, cl, ca, R, T = symbols("f l a cf cl ca R T", real=True)
//
// walk_volume = ((abs(f)*cf)**T+(abs(l)*cl)**T)**(R/T)+(abs(a)*ca)**R
//
// walk_volume.diff(f).simplify() # substitute f for the variable of interest
// ```

fn penalty_function(walk_volume_value: f32) -> f32 {
    walk_volume_value.powi(6)
}

fn penalty_function_derivative(walk_volume_value: f32) -> f32 {
    walk_volume_value.powi(5) * 6.0
}
