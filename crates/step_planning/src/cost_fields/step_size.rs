use types::{
    step::{Step, StepAndSupportFoot},
    support_foot::Side,
    walk_volume_extents::WalkVolumeExtents,
};

pub struct StepSizeField {
    pub walk_volume_coefficients: WalkVolumeCoefficients,
}

#[derive(Clone, Debug)]
pub struct WalkVolumeCoefficients {
    pub forward_cost: f32,
    pub backward_cost: f32,
    pub outward_cost: f32,
    pub inward_cost: f32,
    pub outward_rotation_cost: f32,
    pub inward_rotation_cost: f32,
}

impl WalkVolumeCoefficients {
    pub fn from_extents(extents: &WalkVolumeExtents) -> Self {
        let WalkVolumeExtents {
            forward,
            backward,
            outward,
            inward,
            outward_rotation,
            inward_rotation,
        } = extents;

        Self {
            forward_cost: 1.0 / forward,
            backward_cost: 1.0 / backward,
            outward_cost: 1.0 / outward,
            inward_cost: 1.0 / inward,
            outward_rotation_cost: 1.0 / outward_rotation,
            inward_rotation_cost: 1.0 / inward_rotation,
        }
    }
}

impl WalkVolumeCoefficients {
    fn costs(
        &self,
        StepAndSupportFoot { step, support_foot }: &StepAndSupportFoot<f32>,
    ) -> Step<f32> {
        let Self {
            forward_cost: positive_forward_cost,
            backward_cost: negative_forward_cost,
            outward_cost,
            inward_cost,
            outward_rotation_cost,
            inward_rotation_cost,
        } = self;

        let Step {
            forward,
            left,
            turn,
        } = step;

        let (
            positive_left_cost,
            negative_left_cost,
            clockwise_rotation_cost,
            counterclockwise_rotation_cost,
        ) = match support_foot {
            Side::Left => (
                inward_cost,
                outward_cost,
                outward_rotation_cost,
                inward_rotation_cost,
            ),
            Side::Right => (
                outward_cost,
                inward_cost,
                inward_rotation_cost,
                outward_rotation_cost,
            ),
        };

        let forward_cost =
            positive_negative(*forward, *positive_forward_cost, *negative_forward_cost);
        let left_cost = positive_negative(*left, *positive_left_cost, *negative_left_cost);
        let turn_cost = positive_negative(
            *turn,
            *clockwise_rotation_cost,
            *counterclockwise_rotation_cost,
        );

        Step {
            forward: forward_cost,
            left: left_cost,
            turn: turn_cost,
        }
    }
}

#[inline]
fn positive_negative(value: f32, positive: f32, negative: f32) -> f32 {
    if value.is_sign_positive() {
        positive
    } else {
        negative
    }
}

fn walk_volume(
    step: &StepAndSupportFoot<f32>,
    walk_volume_coefficients: &WalkVolumeCoefficients,
) -> f32 {
    let costs = walk_volume_coefficients.costs(step);

    let normalized_forward = step.step.forward * costs.forward;
    let normalized_left = step.step.left * costs.left;
    let normalized_turn = step.step.turn * costs.turn;

    normalized_forward.powi(2) + normalized_left.powi(2) + normalized_turn.powi(2)
}

fn walk_volume_gradient(
    step: &StepAndSupportFoot<f32>,
    walk_volume_coefficients: &WalkVolumeCoefficients,
) -> Step<f32> {
    let costs = walk_volume_coefficients.costs(step);

    Step {
        forward: 2.0 * costs.forward.powi(2) * step.step.forward,
        left: 2.0 * costs.left.powi(2) * step.step.left,
        turn: 2.0 * costs.turn.powi(2) * step.step.turn,
    }
}

fn penalty_function(walk_volume_value: f32) -> f32 {
    walk_volume_value.powi(6)
}

fn penalty_function_derivative(walk_volume_value: f32) -> f32 {
    walk_volume_value.powi(5) * 6.0
}

impl StepSizeField {
    pub fn cost(&self, step: StepAndSupportFoot<f32>) -> f32 {
        let value = walk_volume(&step, &self.walk_volume_coefficients);

        penalty_function(value)
    }

    pub fn grad(&self, step: StepAndSupportFoot<f32>) -> Step<f32> {
        let value = walk_volume(&step, &self.walk_volume_coefficients);
        let grad = walk_volume_gradient(&step, &self.walk_volume_coefficients);

        grad * penalty_function_derivative(value)
    }
}
