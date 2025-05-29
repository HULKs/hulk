use std::time::{Duration, SystemTime};

use color_eyre::Result;
use nalgebra::DVector;
use serde::{Deserialize, Serialize};

use context_attribute::context;
use coordinate_systems::{Ground, UpcomingSupport};
use framework::{AdditionalOutput, MainOutput};
use linear_algebra::{Isometry2, Orientation2};
use step_planning::geometry::Pose;
use types::{
    motion_command::{MotionCommand, OrientationMode, WalkSpeed},
    parameters::StepPlanningOptimizationParameters,
    planned_path::Path,
    step::Step,
    support_foot::Side,
};
use walking_engine::mode::Mode;

const VARIABLES_PER_STEP: usize = 3;

#[derive(Deserialize, Serialize)]
pub struct StepPlanner {
    last_planned_step: Step,
    last_step_plan: DVector<f32>,
    last_support_foot: Side,
}

#[context]
pub struct CreationContext {
    planned_steps: Parameter<usize, "step_planner.planned_steps">,
}

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
    step_plan: AdditionalOutput<Vec<f32>, "step_plan">,
    step_plan_gradient: AdditionalOutput<Vec<f32>, "step_plan_gradient">,
    step_planning_duration: AdditionalOutput<Duration, "step_planning_duration">,
}

#[context]
#[derive(Default)]
pub struct MainOutputs {
    pub planned_step: MainOutput<Step>,
}

impl StepPlanner {
    pub fn new(context: CreationContext) -> Result<Self> {
        Ok(Self {
            last_planned_step: Step::default(),
            last_step_plan: DVector::zeros(*context.planned_steps * VARIABLES_PER_STEP),
            last_support_foot: Side::Left,
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
            ..
        } = context.motion_command
        else {
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
            self.plan_step(
                path.clone(),
                &mut context,
                *orientation_mode,
                *target_orientation,
            )?
        };

        let elapsed = SystemTime::now().duration_since(earlier).unwrap();

        context
            .step_planning_duration
            .fill_if_subscribed(|| elapsed);

        let step = Step {
            forward: step.forward * context.request_scale.forward,
            left: step.left * context.request_scale.left,
            turn: step.turn * context.request_scale.turn,
        };
        let step = self.clamp_step_size(&mut context, speed, step);

        self.last_planned_step = step;

        Ok(MainOutputs {
            planned_step: step.into(),
        })
    }

    fn clamp_step_size(&self, context: &mut CycleContext, speed: &WalkSpeed, step: Step) -> Step {
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

        context
            .max_step_size_output
            .fill_if_subscribed(|| max_step_size);

        let support_side = if let Mode::Walking(walking) = *context.walking_engine_mode {
            Some(walking.step.plan.support_side)
        } else {
            None
        };
        let (max_turn_left, max_turn_right) = if let Some(support_side) = support_side {
            if support_side == Side::Left {
                (-*context.max_inside_turn, context.max_step_size.turn)
            } else {
                (-context.max_step_size.turn, *context.max_inside_turn)
            }
        } else {
            (-context.max_step_size.turn, context.max_step_size.turn)
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

    fn plan_step(
        &mut self,
        path: Path,
        context: &mut CycleContext,
        orientation_mode: OrientationMode,
        target_orientation: Orientation2<Ground>,
    ) -> Result<Step> {
        let num_variables = context.optimization_parameters.num_steps * VARIABLES_PER_STEP;

        let current_support_foot = context
            .walking_engine_mode
            .support_side()
            .unwrap_or(Side::Left);

        let initial_guess = DVector::zeros(num_variables);

        let (step_plan, gradient) = step_planning_solver::plan_steps(
            path,
            orientation_mode,
            target_orientation,
            upcoming_support_pose_in_ground(context),
            current_support_foot.opposite(),
            initial_guess,
            context.optimization_parameters,
        )?;

        context
            .step_plan
            .fill_if_subscribed(|| step_plan.as_slice().to_vec());

        context
            .step_plan_gradient
            .fill_if_subscribed(|| gradient.as_slice().to_vec());

        let step = Step::from_slice(&step_plan.as_slice()[0..VARIABLES_PER_STEP]);

        self.last_step_plan = step_plan;
        self.last_support_foot = current_support_foot;

        Ok(step)
    }
}

fn upcoming_support_pose_in_ground(context: &CycleContext) -> Pose<f32> {
    let pose = context.ground_to_upcoming_support.inverse().as_pose();

    Pose {
        position: pose.position(),
        orientation: pose.orientation().angle(),
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
