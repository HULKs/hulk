use color_eyre::{eyre::eyre, Result};
use ordered_float::NotNan;
use serde::{Deserialize, Serialize};

use context_attribute::context;
use coordinate_systems::{Ground, UpcomingSupport};
use framework::{AdditionalOutput, MainOutput};
use geometry::direction::Rotate90Degrees;
use linear_algebra::{vector, Isometry2, Orientation2, Pose2};
use step_planning::traits::Project;
use types::{
    motion_command::{MotionCommand, OrientationMode, WalkSpeed},
    parameters::StepPlannerParameters,
    planned_path::PathSegment,
    step::Step,
    support_foot::Side,
};
use walking_engine::anatomic_constraints::AnatomicConstraints;
use walking_engine::mode::Mode;

#[derive(Deserialize, Serialize)]
pub struct StepPlanner {
    last_planned_step: Step,
}

#[context]
pub struct CreationContext {}

#[context]
pub struct CycleContext {
    motion_command: Input<MotionCommand, "motion_command">,

    parameters: Parameter<StepPlannerParameters, "step_planner">,
    ground_to_upcoming_support:
        CyclerState<Isometry2<Ground, UpcomingSupport>, "ground_to_upcoming_support">,
    walking_engine_mode: CyclerState<Mode, "walking_engine_mode">,

    ground_to_upcoming_support_out:
        AdditionalOutput<Isometry2<Ground, UpcomingSupport>, "ground_to_upcoming_support">,
    max_step_size_output: AdditionalOutput<Step, "max_step_size">,
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
        let support_side = if let Mode::Walking(walking) = *context.walking_engine_mode {
            Some(walking.step.plan.support_side)
        } else {
            None
        };

        let parameters = context.parameters;

        let (max_turn_left, max_turn_right) = if let Some(support_side) = support_side {
            if support_side == Side::Left {
                (-parameters.max_inside_turn, parameters.max_step_size.turn)
            } else {
                (-parameters.max_step_size.turn, parameters.max_inside_turn)
            }
        } else {
            (
                -parameters.max_step_size.turn,
                parameters.max_step_size.turn,
            )
        };

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
                    planned_step: Step::default().into(),
                })
            }
        };

        let initial_side_bonus = if self.last_planned_step.forward.abs()
            + self.last_planned_step.left.abs()
            + self.last_planned_step.turn.abs()
            <= f32::EPSILON
        {
            Step {
                forward: 0.0,
                left: parameters.initial_side_bonus,
                turn: 0.0,
            }
        } else {
            Step::default()
        };

        let max_step_size = match speed {
            WalkSpeed::Slow => parameters.max_step_size + parameters.step_size_delta_slow,
            WalkSpeed::Normal => parameters.max_step_size + initial_side_bonus,
            WalkSpeed::Fast => {
                parameters.max_step_size + parameters.step_size_delta_fast + initial_side_bonus
            }
        };

        context
            .max_step_size_output
            .fill_if_subscribed(|| max_step_size);

        let segment = path
            .iter()
            .scan(0.0f32, |distance, segment| {
                let result = if *distance < max_step_size.forward {
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
                let direction = (start_point - arc.circle.center).rotate_90_degrees(arc.direction);
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

        step = Step {
            forward: step.forward * parameters.request_scale.forward,
            left: step.left * parameters.request_scale.left,
            turn: step.turn * parameters.request_scale.turn,
        };

        if let Some(injected_step) = parameters.injected_step {
            step = injected_step;
        }

        let step = clamp_step_to_walk_volume(
            step,
            &max_step_size,
            parameters.max_step_size_backwards,
            parameters.translation_exponent,
            parameters.rotation_exponent,
            max_turn_left,
            max_turn_right,
        );

        self.last_planned_step = step;

        Ok(MainOutputs {
            planned_step: step.into(),
        })
    }
}

pub fn step_plan_greedy<Frame>(
    path: &[PathSegment<Frame>],
    parameters: &StepPlannerParameters,
    mut pose: Pose2<Frame>,
    initial_support_side: Side,
    orientation_mode: OrientationMode<Frame>,
    speed: WalkSpeed,
) -> Result<Vec<(Step, Side)>> {
    let mut steps = Vec::new();
    let mut last_planned_step = Step::default();
    let mut support_side = initial_support_side;

    let destination = match path.last() {
        Some(PathSegment::Arc(arc)) => arc.end_point(),
        Some(PathSegment::LineSegment(segment)) => segment.1,
        None => todo!(),
    };

    for _ in 0..3 {
        let segment = path
            .iter()
            .min_by_key(|segment| {
                NotNan::new((segment.project(pose.position()) - pose.position()).norm_squared())
                    .expect("path distance was NaN")
            })
            .ok_or_else(|| eyre!("empty path provided"))?;

        let target_pose = match segment {
            PathSegment::LineSegment(line_segment) => {
                let direction = line_segment.1 - pose.position();
                let rotation = if direction.norm_squared() < f32::EPSILON {
                    Orientation2::identity()
                } else {
                    let normalized_direction = direction.normalize();
                    Orientation2::from_cos_sin_unchecked(
                        normalized_direction.x(),
                        normalized_direction.y(),
                    )
                };
                Pose2::from_parts(line_segment.1, rotation)
            }
            PathSegment::Arc(arc) => {
                let start_point = arc.project(pose.position());
                let direction = (start_point - arc.circle.center).rotate_90_degrees(arc.direction);
                Pose2::from_parts(
                    start_point + direction,
                    Orientation2::from_vector(direction),
                )
            }
        };

        let step_target = pose.as_transform::<Frame>().inverse() * target_pose;

        let step = Step {
            forward: step_target.position().x(),
            left: step_target.position().y(),
            turn: match orientation_mode {
                OrientationMode::AlignWithPath => step_target.orientation().angle(),
                OrientationMode::Override(orientation) => {
                    (pose.orientation().as_transform::<Frame>().inverse() * orientation).angle()
                }
            },
        };

        let step = clamp_step_size(last_planned_step, parameters, support_side, speed, step);
        let step = step.clamp_to_anatomic_constraints(support_side, 0.1, 4.0);

        let step_translation =
            Isometry2::<Ground, Ground>::from_parts(vector![step.forward, step.left], 0.0);
        let step_rotation = Isometry2::<Ground, Ground>::from_parts(vector![0.0, 0.0], step.turn);

        pose = pose.as_transform() * step_rotation * step_translation.as_pose();
        last_planned_step = step;
        steps.push((step, support_side));
        support_side = support_side.opposite();

        if (destination - pose.position()).norm_squared() > 0.01 {
            continue;
        }

        if let OrientationMode::Override(orientation) = orientation_mode {
            let angle_to_desired_orientation =
                pose.orientation().rotation_to(orientation).angle().abs();
            if angle_to_desired_orientation > 0.01 {
                continue;
            }
        }

        break;
    }

    Ok(steps)
}

fn clamp_step_size(
    last_planned_step: Step,
    parameters: &StepPlannerParameters,
    support_side: Side,
    speed: WalkSpeed,
    step: Step,
) -> Step {
    // TODO rethink this with new step planning (maybe scale with exp(-last_planned_step.left) instead)
    let initial_side_bonus = if last_planned_step.forward.abs()
        + last_planned_step.left.abs()
        + last_planned_step.turn.abs()
        <= f32::EPSILON
    {
        Step {
            forward: 0.0,
            left: parameters.initial_side_bonus,
            turn: 0.0,
        }
    } else {
        Step::default()
    };

    let max_step_size = match speed {
        WalkSpeed::Slow => parameters.max_step_size + parameters.step_size_delta_slow,
        WalkSpeed::Normal => parameters.max_step_size + initial_side_bonus,
        WalkSpeed::Fast => {
            parameters.max_step_size + parameters.step_size_delta_fast + initial_side_bonus
        }
    };

    let (max_turn_left, max_turn_right) = if support_side == Side::Left {
        (-parameters.max_inside_turn, parameters.max_step_size.turn)
    } else {
        (-parameters.max_step_size.turn, parameters.max_inside_turn)
    };

    clamp_step_to_walk_volume(
        step,
        &max_step_size,
        parameters.max_step_size_backwards,
        parameters.translation_exponent,
        parameters.rotation_exponent,
        max_turn_left,
        max_turn_right,
    )
}

fn clamp_step_to_walk_volume(
    request: Step,
    max_step_size: &Step,
    max_step_size_backwards: f32,
    translation_exponent: f32,
    rotation_exponent: f32,
    max_turn_left: f32,
    max_turn_right: f32,
) -> Step {
    // Values in range [-1..1]
    let clamped_turn = request.turn.clamp(max_turn_left, max_turn_right);

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
        max_turn_left,
        max_turn_right,
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
        max_turn_left,
        max_turn_right,
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
    max_turn_left: f32,
    max_turn_right: f32,
) -> f32 {
    let is_walking_forward = request.forward.is_sign_positive();
    let max_forward = if is_walking_forward {
        max_step_size.forward
    } else {
        max_step_size_backwards
    };
    let x = request.forward / max_forward;
    let y = request.left / max_step_size.left;
    let angle = if request.turn.is_sign_positive() {
        request.turn / max_turn_right
    } else {
        request.turn / max_turn_left
    };
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
    max_turn_left: f32,
    max_turn_right: f32,
) -> (f32, f32) {
    let is_walking_forward = request.forward.is_sign_positive();
    let max_forward = if is_walking_forward {
        max_step_size.forward
    } else {
        max_step_size_backwards
    };
    let x = request.forward / max_forward;
    let y = request.left / max_step_size.left;
    let angle = if request.turn.is_sign_positive() {
        request.turn / max_turn_right
    } else {
        request.turn / max_turn_left
    };
    assert!(angle.abs() <= 1.0);
    let scale = ((1.0 - angle.abs().powf(rotation_exponent))
        .powf(translation_exponent / rotation_exponent)
        / (x.abs().powf(translation_exponent) + y.abs().powf(translation_exponent)))
    .powf(1.0 / translation_exponent);
    (request.forward * scale, request.left * scale)
}
