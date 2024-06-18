use std::time::Duration;

use crate::{
    anatomic_constraints::AnatomicConstraints, parameters::Parameters, step_plan::StepPlan,
    stiffness::Stiffness as _, Context,
};

use super::{
    super::{feet::Feet, step_state::StepState},
    stopping::Stopping,
    walking::Walking,
    Mode, WalkTransition,
};
use coordinate_systems::Ground;
use linear_algebra::Point2;
use path_serde::{PathDeserialize, PathIntrospect, PathSerialize};
use serde::{Deserialize, Serialize};
use types::{
    joints::body::BodyJoints, motion_command::KickVariant, motor_commands::MotorCommands,
    step_plan::Step, support_foot::Side,
};

#[derive(
    Clone, Copy, Debug, Serialize, Deserialize, PathSerialize, PathDeserialize, PathIntrospect,
)]
pub struct Catching {
    pub step: StepState,
}

impl Catching {
    pub fn new(context: &Context, support_side: Side) -> Self {
        let parameters = &context.parameters;
        let target_overestimation_factor = context
            .parameters
            .catching_steps
            .target_overestimation_factor;

        let step_duration = parameters.base.step_duration;
        let start_feet =
            Feet::from_joints(context.robot_to_walk, &context.current_joints, support_side);

        let end_feet = catching_end_feet(
            parameters,
            *context.zero_moment_point,
            target_overestimation_factor,
            support_side,
        );
        let max_swing_foot_lift =
            parameters.base.foot_lift_apex + parameters.catching_steps.additional_foot_lift;
        let midpoint = parameters.catching_steps.midpoint;

        let step = StepState {
            plan: StepPlan {
                step_duration,
                start_feet,
                end_feet,
                support_side,
                foot_lift_apex: max_swing_foot_lift,
                midpoint,
            },
            time_since_start: Duration::ZERO,
            gyro_balancing: Default::default(),
            foot_leveling: Default::default(),
        };
        Self { step }
    }

    fn next_step(self, context: &Context) -> Mode {
        let current_step = self.step;

        if context.robot_to_ground.is_none() {
            return Mode::Stopping(Stopping::new(context, current_step.plan.support_side));
        }
        if *context.number_of_frames_zero_moment_point_has_been_outside_support_polygon
            <= context
                .parameters
                .catching_steps
                .catching_step_zero_moment_point_frame_count_threshold
        {
            return Mode::Walking(Walking::new(
                context,
                Step::ZERO,
                current_step.plan.support_side.opposite(),
                Step::ZERO,
            ));
        }
        Mode::Catching(self)
    }
}

fn catching_end_feet(
    parameters: &Parameters,
    zero_moment_point: Point2<Ground>,
    target_overestimation_factor: f32,
    support_side: Side,
) -> Feet {
    let max_adjustment = parameters.catching_steps.max_adjustment;
    Feet::end_from_request(
        parameters,
        Step {
            forward: (zero_moment_point.x() * target_overestimation_factor)
                .clamp(-max_adjustment, max_adjustment),
            left: (zero_moment_point.y() * target_overestimation_factor)
                .clamp(-max_adjustment, max_adjustment),
            turn: 0.0,
        }
        .clamp_to_anatomic_constraints(support_side, parameters.max_inside_turn),
        support_side,
    )
}

impl WalkTransition for Catching {
    fn stand(self, context: &Context) -> Mode {
        let current_step = self.step;
        if current_step.is_support_switched(context) {
            return self.next_step(context);
        }

        Mode::Catching(self)
    }

    fn walk(self, context: &Context, _requested_step: Step) -> Mode {
        let current_step = self.step;
        if current_step.is_support_switched(context) {
            return self.next_step(context);
        }

        Mode::Catching(self)
    }

    fn kick(self, context: &Context, _variant: KickVariant, _side: Side, _strength: f32) -> Mode {
        let current_step = self.step;
        if current_step.is_support_switched(context) {
            return self.next_step(context);
        }

        Mode::Catching(self)
    }
}

impl Catching {
    pub fn compute_commands(&self, context: &Context) -> MotorCommands<BodyJoints> {
        self.step.compute_joints(context).apply_stiffness(
            context.parameters.stiffnesses.leg_stiffness_walk,
            context.parameters.stiffnesses.arm_stiffness,
        )
    }

    pub fn tick(&mut self, context: &Context) {
        self.step.plan.end_feet = catching_end_feet(
            context.parameters,
            *context.zero_moment_point,
            context
                .parameters
                .catching_steps
                .target_overestimation_factor,
            self.step.plan.support_side,
        );
        self.step.tick(context);
    }
}
