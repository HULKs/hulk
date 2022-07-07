use std::time::{Duration, SystemTime};

use approx::relative_eq;
use module_derive::{module, require_some};
use types::{
    BodyJoints, FallDirection, HeadJoints, Joints, JointsCommand, MotionCommand, MotionSelection,
    MotionType, SensorData,
};

use crate::framework::configuration::FallProtection;

pub struct FallProtector {
    start_time: SystemTime,
}

#[module(control)]
#[parameter(path = control.fall_protection, data_type = FallProtection)]
#[input(path = sensor_data, data_type = SensorData)]
#[input(path = motion_selection, data_type = MotionSelection)]
#[input(path = motion_command, data_type = MotionCommand)]
#[main_output(name = fall_protection_command, data_type = JointsCommand)]

impl FallProtector {}

impl FallProtector {
    fn new(_context: NewContext) -> anyhow::Result<Self> {
        Ok(Self {
            start_time: SystemTime::now(),
        })
    }

    fn cycle(&mut self, context: CycleContext) -> anyhow::Result<MainOutputs> {
        let motion_selection = require_some!(context.motion_selection);
        let motion_command = require_some!(context.motion_command);
        let current_positions = require_some!(context.sensor_data).positions;

        let mut head_stiffness = 1.0;

        if motion_selection.current_motion != MotionType::FallProtection {
            self.start_time = SystemTime::now();
            return Ok(MainOutputs {
                fall_protection_command: Some(JointsCommand {
                    positions: current_positions,
                    stiffnesses: Joints::fill(0.8),
                }),
            });
        }

        if self.start_time.elapsed().unwrap() >= Duration::from_millis(500) {
            head_stiffness = 0.5;
        }
        match motion_command {
            MotionCommand::FallProtection {
                direction: FallDirection::Forward,
            } => {
                if relative_eq!(current_positions.head.pitch, -0.672, epsilon = 0.05)
                    && relative_eq!(current_positions.head.yaw.abs(), 0.0, epsilon = 0.05)
                {
                    head_stiffness = context.fall_protection.ground_impact_head_stiffness;
                }
            }
            MotionCommand::FallProtection { .. } => {
                if relative_eq!(current_positions.head.pitch, 0.5149, epsilon = 0.05)
                    && relative_eq!(current_positions.head.yaw.abs(), 0.0, epsilon = 0.05)
                {
                    head_stiffness = context.fall_protection.ground_impact_head_stiffness;
                }
            }
            _ => head_stiffness = context.fall_protection.ground_impact_head_stiffness,
        }

        let stiffnesses = Joints::from_head_and_body(
            HeadJoints::fill(head_stiffness),
            BodyJoints::selective_fill(
                context.fall_protection.arm_stiffness,
                context.fall_protection.leg_stiffness,
            ),
        );

        let fall_protection_command = match motion_command {
            MotionCommand::FallProtection {
                direction: FallDirection::Forward,
            } => Some(JointsCommand {
                positions: Joints::from_head_and_body(
                    HeadJoints {
                        yaw: 0.0,
                        pitch: -0.672,
                    },
                    BodyJoints {
                        left_arm: context.fall_protection.left_arm_positions,
                        right_arm: context.fall_protection.right_arm_positions,
                        left_leg: current_positions.left_leg,
                        right_leg: current_positions.right_leg,
                    },
                ),
                stiffnesses,
            }),
            _ => Some(JointsCommand {
                positions: Joints::from_head_and_body(
                    HeadJoints {
                        yaw: 0.0,
                        pitch: 0.5149,
                    },
                    BodyJoints {
                        left_arm: context.fall_protection.left_arm_positions,
                        right_arm: context.fall_protection.right_arm_positions,
                        left_leg: current_positions.left_leg,
                        right_leg: current_positions.right_leg,
                    },
                ),
                stiffnesses,
            }),
        };

        Ok(MainOutputs {
            fall_protection_command,
        })
    }
}
