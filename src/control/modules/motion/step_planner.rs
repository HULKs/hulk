use macros::{module, require_some};

use crate::types::{MotionCommand, PlannedPath, SensorData, Side, Step, StepPlan, SupportFoot};

pub struct StepPlanner;

#[module(control)]
#[input(path = sensor_data, data_type = SensorData)]
#[input(path = motion_command, data_type = MotionCommand)]
#[input(path = support_foot, data_type = SupportFoot)]
#[input(path = planned_path, data_type = PlannedPath)]
#[parameter(path = control.step_planner.max_step_size, data_type = Step)]
#[parameter(path = control.step_planner.max_step_size_backwards, data_type = f32)]
#[parameter(path = control.step_planner.translation_exponent, data_type = f32)]
#[parameter(path = control.step_planner.rotation_exponent, data_type = f32)]
#[parameter(path = control.step_planner.inside_turn_ratio, data_type = f32)]
#[main_output(data_type = StepPlan)]
impl StepPlanner {}

impl StepPlanner {
    fn new(_context: NewContext) -> anyhow::Result<Self> {
        Ok(Self)
    }

    fn cycle(&mut self, context: CycleContext) -> anyhow::Result<MainOutputs> {
        let motion_command = require_some!(context.motion_command);
        let support_side = require_some!(context.support_foot).support_side;

        let target_pose = match motion_command.motion {
            crate::types::Motion::Walk { .. } => require_some!(context.planned_path).end_pose,
            _ => {
                return Ok(MainOutputs {
                    step_plan: Some(StepPlan {
                        step: Step {
                            forward: 0.0,
                            left: 0.0,
                            turn: 0.0,
                        },
                    }),
                })
            }
        };

        let step = Step {
            forward: target_pose.translation.x,
            left: target_pose.translation.y,
            turn: target_pose.rotation.angle(),
        };

        // TODO : compensate_with_return_offset

        // clamp_step_to_walk_volume
        let step = clamp_step_to_walk_volume(
            step,
            context.max_step_size,
            *context.max_step_size_backwards,
            *context.translation_exponent,
            *context.rotation_exponent,
        );

        // clamp_to_anatomic_constraints
        let step = clamp_to_anatomic_constraints(step, support_side, *context.inside_turn_ratio);

        Ok(MainOutputs {
            step_plan: Some(StepPlan { step }),
        })
    }
}

fn clamp_step_to_walk_volume(
    request: Step,
    max_step_size: &Step,
    max_step_size_backwards: f32,
    translation_exponent: f32,
    rotation_exponent: f32,
) -> Step {
    // Values in range [-1..1]
    let clamped_turn = request.turn.clamp(-max_step_size.turn, max_step_size.turn);
    let request = Step {
        forward: request.forward,
        left: request.left,
        turn: clamped_turn,
    };
    if calculate_walk_volume(
        request,
        max_step_size,
        max_step_size_backwards,
        translation_exponent,
        rotation_exponent,
    ) <= 1.0
    {
        return request;
    }
    // the step has to be scaled to the ellipse
    let (forward, left) = calculate_max_step_size_in_walk_volume(
        request,
        max_step_size,
        max_step_size_backwards,
        translation_exponent,
        rotation_exponent,
    );
    Step {
        forward,
        left,
        turn: request.turn,
    }
}

fn calculate_walk_volume(
    request: Step,
    max_step_size: &Step,
    max_step_size_backwards: f32,
    translation_exponent: f32,
    rotation_exponent: f32,
) -> f32 {
    let is_walking_forward = request.forward.is_sign_positive();
    let max_forward = if is_walking_forward {
        max_step_size.forward
    } else {
        max_step_size_backwards
    };
    let x = request.forward / max_forward;
    let y = request.left / max_step_size.left;
    let angle = request.turn / max_step_size.turn;
    assert!(angle.abs() <= 1.0);
    (x.abs().powf(translation_exponent) + y.abs().powf(translation_exponent))
        .powf(rotation_exponent / translation_exponent)
        + angle.abs().powf(rotation_exponent)
}

fn calculate_max_step_size_in_walk_volume(
    request: Step,
    max_step_size: &Step,
    max_step_size_backwards: f32,
    translation_exponent: f32,
    rotation_exponent: f32,
) -> (f32, f32) {
    let is_walking_forward = request.forward.is_sign_positive();
    let max_forward = if is_walking_forward {
        max_step_size.forward
    } else {
        max_step_size_backwards
    };
    let x = request.forward / max_forward;
    let y = request.left / max_step_size.left;
    let angle = request.turn / max_step_size.turn;
    assert!(angle.abs() <= 1.0);
    let scale = ((1.0 - angle.abs().powf(rotation_exponent))
        .powf(translation_exponent / rotation_exponent)
        / (x.abs().powf(translation_exponent) + y.abs().powf(translation_exponent)))
    .powf(1.0 / translation_exponent);
    (request.forward * scale, request.left * scale)
}

fn clamp_to_anatomic_constraints(
    request: Step,
    support_side: Side,
    inside_turn_ratio: f32,
) -> Step {
    let sideways_direction = if request.left.is_sign_positive() {
        Side::Left
    } else {
        Side::Right
    };
    let clamped_left = if sideways_direction == support_side {
        0.0
    } else {
        request.left
    };
    let turn_direction = if request.turn.is_sign_positive() {
        Side::Left
    } else {
        Side::Right
    };
    let turn_ratio = if turn_direction == support_side {
        inside_turn_ratio
    } else {
        1.0 - inside_turn_ratio
    };
    let clamped_turn = turn_ratio * request.turn;
    Step {
        forward: request.forward,
        left: clamped_left,
        turn: clamped_turn,
    }
}
