use color_eyre::{eyre::eyre, Result};
use coordinate_systems::{Ground, UpcomingSupport};
use serde::{Deserialize, Serialize};

use context_attribute::context;
use framework::{AdditionalOutput, MainOutput};
use linear_algebra::{Isometry2, Orientation2, Pose2};
use types::{
    motion_command::{MotionCommand, OrientationMode, WalkSpeed},
    planned_path::PathSegment,
    step::Step,
};

#[derive(Deserialize, Serialize)]
pub struct StepPlanner {
    last_planned_step: Step,
}

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
    initial_side_bonus: Parameter<f32, "step_planner.initial_side_bonus">,

    ground_to_upcoming_support:
        CyclerState<Isometry2<Ground, UpcomingSupport>, "ground_to_upcoming_support">,

    ground_to_upcoming_support_out:
        AdditionalOutput<Isometry2<Ground, UpcomingSupport>, "ground_to_upcoming_support">,
}

#[context]
#[derive(Default)]
pub struct MainOutputs {
    pub planned_step: MainOutput<Step>,
}

impl StepPlanner {
    pub fn new(_context: CreationContext) -> Result<Self> {
        Ok(Self {
            last_planned_step: Step::default(),
        })
    }

    pub fn cycle(&mut self, mut context: CycleContext) -> Result<MainOutputs> {
        context
            .ground_to_upcoming_support_out
            .fill_if_subscribed(|| *context.ground_to_upcoming_support);

        let (path, orientation_mode, speed) = match context.motion_command {
            MotionCommand::Walk {
                path,
                orientation_mode,
                speed,
                ..
            } => (path, orientation_mode, speed),
            _ => {
                return Ok(MainOutputs {
                    planned_step: Step {
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
            PathSegment::Arc(arc) => {
                let start_point = arc.start_point();
                let direction = arc
                    .direction
                    .rotate_vector_90_degrees(start_point - arc.circle.center);
                Pose2::from_parts(
                    start_point + direction,
                    Orientation2::from_vector(direction),
                )
            }
        };

        let step_target = *context.ground_to_upcoming_support * target_pose;

        let mut step = Step {
            forward: step_target.position().x(),
            left: step_target.position().y(),
            turn: match orientation_mode {
                OrientationMode::AlignWithPath => step_target.orientation().angle(),
                OrientationMode::Override(orientation) => {
                    let ground_to_upcoming_support = context
                        .ground_to_upcoming_support
                        .orientation()
                        .as_transform();
                    (ground_to_upcoming_support * orientation).angle()
                }
            },
        };

        if let Some(injected_step) = context.injected_step {
            step = *injected_step;
        }

        let initial_side_bonus = if self.last_planned_step.left == 0.0 {
            Step {
                forward: 0.0,
                left: *context.initial_side_bonus,
                turn: 0.0,
            }
        } else {
            Step::default()
        };

        let max_step_size = match speed {
            WalkSpeed::Slow => *context.max_step_size + *context.step_size_delta_slow,
            WalkSpeed::Normal => *context.max_step_size + initial_side_bonus,
            WalkSpeed::Fast => {
                *context.max_step_size + *context.step_size_delta_fast + initial_side_bonus
            }
        };

        let step = clamp_step_to_walk_volume(
            step,
            &max_step_size,
            *context.max_step_size_backwards,
            *context.translation_exponent,
            *context.rotation_exponent,
        );

        self.last_planned_step = step;

        Ok(MainOutputs {
            planned_step: step.into(),
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
