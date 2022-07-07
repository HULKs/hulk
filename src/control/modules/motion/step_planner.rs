use module_derive::{module, require_some};
use nalgebra::{Isometry2, UnitComplex};
use types::{MotionCommand, OrientationMode, PathSegment, SensorData, Side, Step, SupportFoot};

pub struct StepPlanner;

#[module(control)]
#[input(path = sensor_data, data_type = SensorData)]
#[input(path = motion_command, data_type = MotionCommand)]
#[input(path = support_foot, data_type = SupportFoot)]
#[persistent_state(path = walk_return_offset, data_type = Step)]
#[parameter(path = control.step_planner.injected_step, data_type = Option<Step>)]
#[parameter(path = control.step_planner.max_step_size, data_type = Step)]
#[parameter(path = control.step_planner.max_step_size_backwards, data_type = f32)]
#[parameter(path = control.step_planner.translation_exponent, data_type = f32)]
#[parameter(path = control.step_planner.rotation_exponent, data_type = f32)]
#[parameter(path = control.step_planner.inside_turn_ratio, data_type = f32)]
#[main_output(name = step_plan, data_type = Step)]
impl StepPlanner {}

impl StepPlanner {
    fn new(_context: NewContext) -> anyhow::Result<Self> {
        Ok(Self)
    }

    fn cycle(&mut self, context: CycleContext) -> anyhow::Result<MainOutputs> {
        let motion_command = require_some!(context.motion_command);
        let support_side = require_some!(require_some!(context.support_foot).support_side);

        let (path, orientation_mode) = match motion_command {
            MotionCommand::Walk {
                path,
                orientation_mode,
                ..
            } => (path, orientation_mode),
            _ => {
                return Ok(MainOutputs {
                    step_plan: Some(Step {
                        forward: 0.0,
                        left: 0.0,
                        turn: 0.0,
                    }),
                })
            }
        };

        let segment = path
            .iter()
            .scan(0.0f32, |distance, segment| {
                let result = if *distance < context.max_step_size.forward {
                    Some(segment)
                } else {
                    None
                };
                *distance += segment.length();
                result
            })
            .last()
            .ok_or_else(|| anyhow::anyhow!("Empty path provided"))?;

        let target_pose = match segment {
            PathSegment::LineSegment(line_segment) => {
                let direction = line_segment.1;
                let rotation = if direction.coords.norm_squared() < f32::EPSILON {
                    UnitComplex::identity()
                } else {
                    UnitComplex::from_cos_sin_unchecked(direction.x, direction.y)
                };
                Isometry2::<f32>::from_parts(line_segment.1.into(), rotation)
            }
            PathSegment::Arc(arc, orientation) => {
                let direction = orientation
                    .rotate_vector_90_degrees(arc.start - arc.circle.center)
                    .normalize();
                Isometry2::<f32>::from_parts(
                    (arc.start + direction * context.max_step_size.forward).into(),
                    UnitComplex::from_cos_sin_unchecked(direction.x, direction.y),
                )
            }
        };

        let mut step = Step {
            forward: target_pose.translation.x,
            left: target_pose.translation.y,
            turn: match orientation_mode {
                OrientationMode::AlignWithPath => target_pose.rotation,
                OrientationMode::Override(orientation) => *orientation,
            }
            .angle(),
        };

        if let Some(injected_step) = context.injected_step {
            step = *injected_step;
        }

        let step = compensate_with_return_offset(step, *context.walk_return_offset);

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
            step_plan: Some(step),
        })
    }
}

fn compensate_with_return_offset(step: Step, walk_return_offset: Step) -> Step {
    step - walk_return_offset
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
    assert!(angle.abs() <= 1.0, "angle was {}", angle);
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
