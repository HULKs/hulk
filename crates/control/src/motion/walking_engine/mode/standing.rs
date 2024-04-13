use std::time::Duration;

use crate::motion::walking_engine::{
    feet::Feet, step_plan::StepPlan, step_state::StepState, stiffness::Stiffness,
};

use super::{super::CycleContext, Mode, Starting, WalkTransition};
use serde::{Deserialize, Serialize};
use serialize_hierarchy::SerializeHierarchy;
use types::{
    joints::body::BodyJoints, motion_command::KickVariant, motor_commands::MotorCommands,
    step_plan::Step, support_foot::Side, walking_engine::WalkingEngineParameters,
};

#[derive(Default, Clone, Copy, Debug, Serialize, Deserialize, SerializeHierarchy)]
pub struct Standing {}

impl WalkTransition for Standing {
    fn stand(self, _context: &CycleContext, _joints: &BodyJoints) -> Mode {
        Mode::Standing(Standing {})
    }

    fn walk(self, context: &CycleContext, joints: &BodyJoints, step: Step) -> Mode {
        let is_requested_step_towards_left = step.left.is_sign_positive();
        let support_side = if is_requested_step_towards_left {
            Side::Left
        } else {
            Side::Right
        };
        Mode::Starting(Starting::new(context, support_side, joints))
    }

    fn kick(
        self,
        context: &CycleContext,
        joints: &BodyJoints,
        _variant: KickVariant,
        kicking_side: Side,
        _strength: f32,
    ) -> Mode {
        // TODO: is this the correct side?
        let support_side = if kicking_side == Side::Left {
            Side::Left
        } else {
            Side::Right
        };
        Mode::Starting(Starting::new(context, support_side, joints))
    }
}

impl Standing {
    pub fn compute_commands(
        &self,
        parameters: &WalkingEngineParameters,
    ) -> MotorCommands<BodyJoints> {
        let feet = Feet::end_from_request(parameters, Step::ZERO, Side::Left);

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
        zero_step_state.compute_joints(parameters).apply_stiffness(
            parameters.stiffnesses.leg_stiffness_stand,
            parameters.stiffnesses.arm_stiffness,
        )
    }

    pub fn tick(&mut self, _context: &CycleContext, _gyro: nalgebra::Vector3<f32>) {}
}
