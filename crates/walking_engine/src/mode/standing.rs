use std::time::Duration;

use path_serde::{PathDeserialize, PathIntrospect, PathSerialize};
use serde::{Deserialize, Serialize};
use types::{
    joints::body::BodyJoints, motion_command::KickVariant, motor_commands::MotorCommands,
    step_plan::Step, support_foot::Side,
};

use crate::{
    feet::Feet, step_plan::StepPlan, step_state::StepState, stiffness::Stiffness as _, Context,
    WalkTransition,
};

use super::{starting::Starting, Mode};

#[derive(
    Default,
    Clone,
    Copy,
    Debug,
    Serialize,
    Deserialize,
    PathSerialize,
    PathDeserialize,
    PathIntrospect,
)]
pub struct Standing {}

impl WalkTransition for Standing {
    fn stand(self, _context: &Context) -> Mode {
        Mode::Standing(Standing {})
    }

    fn walk(self, context: &Context, step: Step) -> Mode {
        let is_requested_step_towards_left = step.left.is_sign_positive();
        let support_side = if is_requested_step_towards_left {
            Side::Left
        } else {
            Side::Right
        };
        Mode::Starting(Starting::new(context, support_side))
    }

    fn kick(
        self,
        context: &Context,
        _variant: KickVariant,
        kicking_side: Side,
        _strength: f32,
    ) -> Mode {
        let support_side = if kicking_side == Side::Left {
            Side::Left
        } else {
            Side::Right
        };
        Mode::Starting(Starting::new(context, support_side))
    }
}

impl Standing {
    pub fn compute_commands(&self, context: &Context) -> MotorCommands<BodyJoints> {
        let feet = Feet::end_from_request(context.parameters, Step::ZERO, Side::Left);

        let zero_step_state = StepState {
            plan: StepPlan {
                step_duration: Duration::from_secs(1),
                start_feet: feet,
                end_feet: feet,
                support_side: Side::Left,
                foot_lift_apex: 0.0,
                midpoint: 0.5,
            },
            time_since_start: Duration::ZERO,
            gyro_balancing: Default::default(),
            foot_leveling: Default::default(),
        };
        zero_step_state.compute_joints(context).apply_stiffness(
            context.parameters.stiffnesses.leg_stiffness_stand,
            context.parameters.stiffnesses.arm_stiffness,
        )
    }

    pub fn tick(&mut self, _context: &Context) {}
}
