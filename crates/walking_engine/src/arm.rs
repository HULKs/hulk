use splines::Interpolate as _;
use types::{
    joints::{arm::ArmJoints, body::BodyJoints},
    motor_commands::MotorCommands,
    obstacle_avoiding_arms::ArmCommand,
};

use crate::Context;

pub trait ArmOverrides {
    fn override_with_arms(self, cotext: &Context) -> Self;
}

impl ArmOverrides for MotorCommands<BodyJoints> {
    fn override_with_arms(self, context: &Context) -> Self {
        let left_swinging_arm = self.positions.left_arm;
        let right_swinging_arm = self.positions.right_arm;

        let left_positions =
            compute_joints(&context.obstacle_avoiding_arms.left_arm, left_swinging_arm);
        let right_positions = compute_joints(
            &context.obstacle_avoiding_arms.right_arm,
            right_swinging_arm,
        );

        let positions = BodyJoints {
            left_arm: left_positions,
            right_arm: right_positions,
            left_leg: self.positions.left_leg,
            right_leg: self.positions.right_leg,
        };

        Self { positions, ..self }
    }
}

fn compute_joints(arm_command: &ArmCommand, swinging_arms: ArmJoints) -> ArmJoints {
    match arm_command {
        ArmCommand::Swing => swinging_arms,
        ArmCommand::Activating {
            influence,
            positions,
        } => ArmJoints::lerp(*influence, swinging_arms, *positions),
        ArmCommand::Active { positions } => *positions,
    }
}
