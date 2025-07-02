use std::time::{Duration, SystemTime};

use color_eyre::{eyre::eyre, Result};
use geometry::{direction::Rotate90Degrees, look_at::LookAt};
use ordered_float::NotNan;
use serde::{Deserialize, Serialize};

use context_attribute::context;
use coordinate_systems::{Ground, UpcomingSupport};
use framework::{AdditionalOutput, MainOutput};
use linear_algebra::{vector, Isometry2, Orientation2, Point2, Pose2};
use step_planning::{
    geometry::{angle::Angle, normalized_step::NormalizedStep, pose::Pose},
    step_plan::StepPlan,
    traits::{EndPoints, Project},
};
use types::{
    motion_command::{MotionCommand, OrientationMode, WalkSpeed},
    parameters::StepPlanningOptimizationParameters,
    planned_path::{Path, PathSegment},
    step::Step,
    support_foot::Side,
};
use walking_engine::mode::Mode;

const VARIABLES_PER_STEP: usize = 3;

#[derive(Deserialize, Serialize)]
pub struct StepPlanner {
    last_planned_step: Step,
    last_step_plan: Option<Vec<f64>>,
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
    max_inside_turn: Parameter<f32, "step_planner.max_inside_turn">,
    rotation_exponent: Parameter<f32, "step_planner.rotation_exponent">,
    translation_exponent: Parameter<f32, "step_planner.translation_exponent">,
    initial_side_bonus: Parameter<f32, "step_planner.initial_side_bonus">,
    request_scale: Parameter<Step, "step_planner.request_scale">,
    optimization_parameters:
        Parameter<StepPlanningOptimizationParameters, "step_planner.optimization_parameters">,

    ground_to_upcoming_support:
        CyclerState<Isometry2<Ground, UpcomingSupport>, "ground_to_upcoming_support">,
    walking_engine_mode: CyclerState<Mode, "walking_engine_mode">,

    ground_to_upcoming_support_out:
        AdditionalOutput<Isometry2<Ground, UpcomingSupport>, "ground_to_upcoming_support">,
    max_step_size_output: AdditionalOutput<Step, "max_step_size">,
    direct_step: AdditionalOutput<Step, "direct_step">,
    step_plan: AdditionalOutput<Vec<Step>, "step_plan">,
    step_plan_greedy: AdditionalOutput<Vec<Step>, "step_plan_greedy">,
    step_plan_gradient: AdditionalOutput<Vec<f32>, "step_plan_gradient">,
    step_plan_cost: AdditionalOutput<f32, "step_plan_cost">,
    current_support_side: AdditionalOutput<Option<Side>, "current_support_side">,
    step_planning_duration: AdditionalOutput<Duration, "step_planning_duration">,
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
            last_step_plan: None,
        })
    }

    pub fn cycle(&mut self, mut context: CycleContext) -> Result<MainOutputs> {
        context
            .ground_to_upcoming_support_out
            .fill_if_subscribed(|| *context.ground_to_upcoming_support);

        let MotionCommand::Walk {
            path,
            orientation_mode,
            speed,
            target_orientation,
            distance_to_be_aligned,
            ..
        } = context.motion_command
        else {
            self.last_step_plan = None;
            return Ok(MainOutputs {
                planned_step: Step {
                    forward: 0.0,
                    left: 0.0,
                    turn: 0.0,
                }
                .into(),
            });
        };

        let earlier = SystemTime::now();

        let step = if let Some(injected_step) = context.injected_step {
            *injected_step
        } else {
            let step_plan_greedy = step_plan_greedy(
                path,
                &mut context,
                *orientation_mode,
                *target_orientation,
                *speed,
            )
            .expect("greedy step planning failed");
            context
                .step_plan_greedy
                .fill_if_subscribed(|| step_plan_greedy);

            self.plan_step(
                path,
                &mut context,
                *orientation_mode,
                *target_orientation,
                *distance_to_be_aligned,
            )?
        };

        let elapsed = SystemTime::now().duration_since(earlier).unwrap();

        context
            .step_planning_duration
            .fill_if_subscribed(|| elapsed);

        let next_support_side = context
            .walking_engine_mode
            .support_side()
            .unwrap_or(Side::Left)
            .opposite();

        let step = Step {
            forward: step.forward * context.request_scale.forward,
            left: step.left * context.request_scale.left,
            turn: step.turn * context.request_scale.turn,
        };

        let step = clamp_step_size(
            self.last_planned_step,
            &mut context,
            next_support_side,
            *speed,
            step,
        );

        self.last_planned_step = step;

        Ok(MainOutputs {
            planned_step: step.into(),
        })
    }

    fn plan_step(
        &mut self,
        path: &Path,
        context: &mut CycleContext,
        orientation_mode: OrientationMode,
        target_orientation: Orientation2<Ground>,
        distance_to_be_aligned: f32,
    ) -> Result<Step> {
        let num_variables = context.optimization_parameters.num_steps * VARIABLES_PER_STEP;

        let current_support_side = context.walking_engine_mode.support_side();

        context
            .current_support_side
            .fill_if_subscribed(|| current_support_side);

        let next_support_side = current_support_side.unwrap_or(Side::Left).opposite();

        let target_point = path.end_point();
        let target_pose = Pose2::from_parts(target_point, target_orientation);

        let target_pose_in_upcoming_support = *context.ground_to_upcoming_support * target_pose;
        let direct_step_to_target = Step::from_pose(target_pose_in_upcoming_support);
        let normalized_direct_step_to_target = NormalizedStep::from_step(
            direct_step_to_target,
            &context.optimization_parameters.walk_volume_extents,
            next_support_side,
        );

        if normalized_direct_step_to_target.is_inside_walk_volume() {
            context
                .direct_step
                .fill_if_subscribed(|| direct_step_to_target);

            self.last_step_plan = None;

            return Ok(direct_step_to_target);
        }

        let variables = self.last_step_plan.get_or_insert(vec![0.0; num_variables]);

        let (gradient, cost) = step_planning_solver::plan_steps(
            path,
            orientation_mode,
            target_orientation,
            distance_to_be_aligned,
            upcoming_support_pose_in_ground(context),
            next_support_side,
            variables.as_mut_slice(),
            context.optimization_parameters,
        )?;

        let variables_f32: Vec<f32> = variables.iter().map(|&x| x as f32).collect();

        let step_plan: Vec<Step> = StepPlan::from(variables_f32.as_slice())
            .steps()
            .scan(next_support_side, |support_side, step| {
                let result = step.unnormalize(
                    &context.optimization_parameters.walk_volume_extents,
                    *support_side,
                );
                *support_side = support_side.opposite();

                Some(result)
            })
            .collect();

        let next_step = *step_plan.first().unwrap();

        context.step_plan.fill_if_subscribed(|| step_plan);

        context
            .step_plan_gradient
            .fill_if_subscribed(|| gradient.as_slice().to_vec());

        context.step_plan_cost.fill_if_subscribed(|| cost);

        Ok(next_step)
    }
}

fn clamp_step_size(
    last_planned_step: Step,
    context: &mut CycleContext,
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

    context
        .max_step_size_output
        .fill_if_subscribed(|| max_step_size);

    let (max_turn_left, max_turn_right) = if support_side == Side::Left {
        (-*context.max_inside_turn, context.max_step_size.turn)
    } else {
        (-context.max_step_size.turn, *context.max_inside_turn)
    };

    clamp_step_to_walk_volume(
        step,
        &max_step_size,
        *context.max_step_size_backwards,
        *context.translation_exponent,
        *context.rotation_exponent,
        max_turn_left,
        max_turn_right,
    )
}

fn step_plan_greedy(
    path: &Path,
    context: &mut CycleContext,
    orientation_mode: OrientationMode,
    _target_orientation: Orientation2<Ground>,
    speed: WalkSpeed,
) -> Result<Vec<Step>> {
    let mut pose = context.ground_to_upcoming_support.inverse().as_pose();
    let mut steps = Vec::new();
    let mut last_planned_step = Step::default();
    let mut support_side = context
        .walking_engine_mode
        .support_side()
        .unwrap_or(Side::Left)
        .opposite();

    for _ in 0..context.optimization_parameters.num_steps {
        let segment = path
            .segments
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
                // let start_point = arc.start_point();
                let start_point = arc.project(pose.position());
                let direction = (start_point - arc.circle.center).rotate_90_degrees(arc.direction);
                Pose2::from_parts(
                    start_point + direction,
                    Orientation2::from_vector(direction),
                )
            }
        };

        let step_target = pose.as_transform::<Ground>().inverse() * target_pose;

        let step = Step {
            forward: step_target.position().x(),
            left: step_target.position().y(),
            turn: match orientation_mode {
                OrientationMode::Unspecified => step_target.orientation().angle(),
                OrientationMode::LookTowards(orientation) => {
                    (pose.orientation().as_transform::<Ground>().inverse() * orientation).angle()
                }
                OrientationMode::LookAt(target) => Point2::origin()
                    .look_at(&(pose.as_transform::<Ground>().inverse() * target))
                    .angle(),
            },
        };

        let step = clamp_step_size(last_planned_step, context, support_side, speed, step);

        let step_translation =
            Isometry2::<Ground, Ground>::from_parts(vector![step.forward, step.left], 0.0);
        let step_rotation = Isometry2::<Ground, Ground>::from_parts(vector![0.0, 0.0], step.turn);

        pose = pose.as_transform() * step_rotation * step_translation.as_pose();
        last_planned_step = step;
        support_side = support_side.opposite();

        steps.push(step);
    }

    Ok(steps)
}

fn upcoming_support_pose_in_ground(context: &CycleContext) -> Pose<f32> {
    let pose = context.ground_to_upcoming_support.inverse().as_pose();

    Pose {
        position: pose.position(),
        orientation: Angle(pose.orientation().angle()),
    }
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
