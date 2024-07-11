use color_eyre::{eyre::eyre, Result};
use serde::{Deserialize, Serialize};

use context_attribute::context;
use framework::MainOutput;
use linear_algebra::{Orientation2, Pose2};
use types::{
    motion_command::{MotionCommand, OrientationMode, WalkSpeed},
    planned_path::PathSegment,
    step_plan::Step,
};

#[derive(Deserialize, Serialize)]
pub struct StepPlanner {}

#[context]
pub struct CreationContext {}

#[context]
pub struct CycleContext {
    motion_command: Input<MotionCommand, "motion_command">,

    injected_step: Parameter<Option<Step>, "step_planner.injected_step?">,
    max_step_size: Parameter<Step, "step_planner.max_step_size">,
    step_size_delta_slow: Parameter<Step, "step_planner.step_size_delta_slow">,
    step_size_delta_fast: Parameter<Step, "step_planner.step_size_delta_fast">,
    max_step_size_backwards: Parameter<f32, "step_planner.max_step_size_backwards">,
    rotation_exponent: Parameter<f32, "step_planner.rotation_exponent">,
    translation_exponent: Parameter<f32, "step_planner.translation_exponent">,

    walk_return_offset: CyclerState<Step, "walk_return_offset">,
}

#[context]
#[derive(Default)]
pub struct MainOutputs {
    pub step_plan: MainOutput<Step>,
}

impl StepPlanner {
    pub fn new(_context: CreationContext) -> Result<Self> {
        Ok(Self {})
    }

    pub fn cycle(&mut self, context: CycleContext) -> Result<MainOutputs> {
        let (path, orientation_mode, speed) = match context.motion_command {
            MotionCommand::Walk {
                path,
                orientation_mode,
                speed,
                ..
            } => (path, orientation_mode, speed),
            _ => {
                return Ok(MainOutputs {
                    step_plan: Step {
                        forward: 0.0,
                        left: 0.0,
                        turn: 0.0,
                    }
                    .into(),
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
            .ok_or_else(|| eyre!("empty path provided"))?;

        let target_pose = match segment {
            PathSegment::LineSegment(line_segment) => {
                let direction = line_segment.1;
                let rotation = if direction.coords().norm_squared() < f32::EPSILON {
                    Orientation2::identity()
                } else {
                    let normalized_direction = direction.coords().normalize();
                    Orientation2::from_cos_sin_unchecked(
                        normalized_direction.x(),
                        normalized_direction.y(),
                    )
                };
                Pose2::from_parts(line_segment.1, rotation)
            }
            PathSegment::Arc(arc, orientation) => {
                let direction = orientation.rotate_vector_90_degrees(arc.start - arc.circle.center);
                Pose2::from_parts(
                    arc.start + direction * 1.0,
                    Orientation2::from_vector(direction),
                )
            }
        };

        let mut step = Step {
            forward: target_pose.position().x(),
            left: target_pose.position().y(),
            turn: match orientation_mode {
                OrientationMode::AlignWithPath => target_pose.orientation().angle(),
                OrientationMode::Override(orientation) => orientation.angle(),
            },
        };

        if let Some(injected_step) = context.injected_step {
            step = *injected_step;
        }

        let max_step_size = match speed {
            WalkSpeed::Slow => *context.max_step_size + *context.step_size_delta_slow,
            WalkSpeed::Normal => *context.max_step_size,
            WalkSpeed::Fast => *context.max_step_size + *context.step_size_delta_fast,
        };
        let step = compensate_with_return_offset(step, *context.walk_return_offset);
        let step = clamp_step_to_walk_volume(
            step,
            &max_step_size,
            *context.max_step_size_backwards,
            *context.translation_exponent,
            *context.rotation_exponent,
        );

        Ok(MainOutputs {
            step_plan: step.into(),
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
    assert!(angle.abs() <= 1.0, "angle was {angle}");
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
