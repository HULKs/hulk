use super::{
    super::{kicking::KickState, step_state::StepState, CycleContext},
    catching::{is_in_support_polygon, Catching},
    kicking::Kicking,
    stopping::Stopping,
    Mode, WalkTransition,
};
use serde::{Deserialize, Serialize};
use serialize_hierarchy::SerializeHierarchy;
use types::{
    joints::body::BodyJoints, motion_command::KickVariant, motor_commands::MotorCommands,
    step_plan::Step, support_foot::Side, walking_engine::WalkingEngineParameters,
};

use crate::motion::walking_engine::{step_plan::StepPlan, stiffness::Stiffness};

#[derive(Clone, Copy, Debug, Serialize, Deserialize, SerializeHierarchy)]
pub struct Walking {
    pub step: StepState,
    pub requested_step: Step,
}

impl Walking {
    pub fn new(
        context: &CycleContext,
        requested_step: Step,
        support_side: Side,
        joints: &BodyJoints,
        last_requested_step: Step,
    ) -> Self {
        let requested_step = Step {
            forward: last_requested_step.forward
                + (requested_step.forward - last_requested_step.forward)
                    .min(context.parameters.max_forward_acceleration),
            left: requested_step.left,
            turn: requested_step.turn,
        };
        let plan =
            StepPlan::new_from_request(context.parameters, requested_step, support_side, joints);
        let step = StepState::new(plan);
        Self {
            step,
            requested_step,
        }
    }
}

impl WalkTransition for Walking {
    fn stand(self, context: &CycleContext, joints: &BodyJoints) -> Mode {
        let current_step = self.step;
        if current_step.is_support_switched(context)
            || current_step.is_timeouted(context.parameters)
        {
            return Mode::Stopping(Stopping::new(
                context,
                current_step.plan.support_side.opposite(),
                joints,
            ));
        }

        Mode::Walking(self)
    }

    fn walk(self, context: &CycleContext, joints: &BodyJoints, requested_step: Step) -> Mode {
        let current_step = self.step;

        let Some(&robot_to_ground) = context.robot_to_ground else {
            return Mode::Stopping(Stopping::new(
                context,
                current_step.plan.support_side,
                joints,
            ));
        };

        if !is_in_support_polygon(
            &context.parameters.catching_steps,
            joints,
            robot_to_ground,
            *context.center_of_mass,
        ) {
            return Mode::Catching(Catching::new(
                context,
                current_step.plan.support_side,
                joints,
                robot_to_ground,
            ));
        };

        if current_step.is_timeouted(context.parameters) {
            return Mode::Walking(Walking::new(
                context,
                Step::ZERO,
                current_step.plan.support_side.opposite(),
                joints,
                self.requested_step,
            ));
        }

        if current_step.is_support_switched(context) {
            return Mode::Walking(Walking::new(
                context,
                requested_step,
                current_step.plan.support_side.opposite(),
                joints,
                self.requested_step,
            ));
        }

        Mode::Walking(self)
    }

    fn kick(
        self,
        context: &CycleContext,
        joints: &BodyJoints,
        variant: KickVariant,
        kicking_side: Side,
        strength: f32,
    ) -> Mode {
        let current_step = self.step;

        if current_step.is_timeouted(context.parameters) {
            return Mode::Walking(Walking::new(
                context,
                Step::ZERO,
                current_step.plan.support_side.opposite(),
                joints,
                self.requested_step,
            ));
        }

        if current_step.is_support_switched(context) {
            let next_support_side = current_step.plan.support_side.opposite();
            // TODO: all kicks require a pre-step
            if next_support_side != kicking_side {
                return Mode::Walking(Walking::new(
                    context,
                    Step::ZERO,
                    next_support_side,
                    joints,
                    self.requested_step,
                ));
            }

            return Mode::Kicking(Kicking::new(
                context,
                KickState::new(variant, kicking_side, strength),
                next_support_side,
                joints,
            ));
        }

        Mode::Walking(self)
    }
}

impl Walking {
    pub fn compute_commands(
        &self,
        parameters: &WalkingEngineParameters,
    ) -> MotorCommands<BodyJoints> {
        self.step.compute_joints(parameters).apply_stiffness(
            parameters.stiffnesses.leg_stiffness_walk,
            parameters.stiffnesses.arm_stiffness,
        )
    }

    pub fn tick(&mut self, context: &CycleContext, gyro: nalgebra::Vector3<f32>) {
        self.step.tick(context, gyro);
    }
}
