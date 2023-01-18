use std::time::{Duration, SystemTime, UNIX_EPOCH};

use approx::relative_eq;
use color_eyre::Result;
use context_attribute::context;
use framework::MainOutput;
use types::{
    configuration::FallProtection, BodyJoints, CycleTime, FallDirection, HeadJoints, Joints,
    JointsCommand, MotionCommand, MotionSelection, MotionType, SensorData,
};

pub struct FallProtector {
    start_time: SystemTime,
}

#[context]
pub struct CreationContext {
    pub fall_protection: Parameter<FallProtection, "control.fall_protection">,
}

#[context]
pub struct CycleContext {
    pub motion_command: Input<MotionCommand, "motion_command">,
    pub motion_selection: Input<MotionSelection, "motion_selection">,
    pub sensor_data: Input<SensorData, "sensor_data">,
    pub cycle_time: Input<CycleTime, "cycle_time">,

    pub fall_protection: Parameter<FallProtection, "control.fall_protection">,
}

#[context]
#[derive(Default)]
pub struct MainOutputs {
    pub fall_protection_command: MainOutput<JointsCommand>,
}

impl FallProtector {
    pub fn new(_context: CreationContext) -> Result<Self> {
        Ok(Self {
            start_time: UNIX_EPOCH,
        })
    }

    pub fn cycle(&mut self, context: CycleContext) -> Result<MainOutputs> {
        let current_positions = context.sensor_data.positions;
        let mut head_stiffness = 1.0;

        if context.motion_selection.current_motion != MotionType::FallProtection {
            self.start_time = context.cycle_time.start_time;
            return Ok(MainOutputs {
                fall_protection_command: JointsCommand {
                    positions: current_positions,
                    stiffnesses: Joints::fill(0.8),
                }
                .into(),
            });
        }

        if self.start_time.elapsed().unwrap() >= Duration::from_millis(500) {
            head_stiffness = 0.5;
        }
        match context.motion_command {
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

        let fall_protection_command = match context.motion_command {
            MotionCommand::FallProtection {
                direction: FallDirection::Forward,
            } => JointsCommand {
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
            },
            _ => JointsCommand {
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
            },
        };

        Ok(MainOutputs {
            fall_protection_command: fall_protection_command.into(),
        })
    }
}
