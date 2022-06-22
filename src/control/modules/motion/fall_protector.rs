use std::time::{Duration, SystemTime};

use approx::relative_eq;
use macros::{module, require_some};

use crate::{
    framework::configuration::FallProtectionParameters,
    types::{
        BodyJoints, FallDirection, HeadJoints, Joints, JointsCommand, Motion, MotionCommand,
        MotionSelection, MotionType, SensorData,
    },
};

pub struct FallProtector {
    start_time: SystemTime,
}

#[module(control)]
#[parameter(name = fall_protection_parameters, path = control.fall_protection_parameters, data_type = FallProtectionParameters)]
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

        let default_output = Ok(MainOutputs {
            fall_protection_command: Some(JointsCommand {
                positions: current_positions,
                stiffnesses: Joints::fill(0.8),
            }),
        });

        if motion_selection.current_motion != MotionType::FallProtection {
            self.start_time = SystemTime::now();
            return default_output;
        }

        if self.start_time.elapsed().unwrap() >= Duration::from_millis(500) {
            head_stiffness = 0.5;
        }
        match motion_command.motion {
            Motion::FallProtection {
                direction: FallDirection::Forward,
            } => {
                if relative_eq!(current_positions.head.pitch, -0.672, epsilon = 0.05)
                    && relative_eq!(current_positions.head.yaw.abs(), 0.0, epsilon = 0.05)
                {
                    head_stiffness = context
                        .fall_protection_parameters
                        .ground_impact_head_stiffness;
                }
            }
            Motion::FallProtection { .. } => {
                if relative_eq!(current_positions.head.pitch, 0.5149, epsilon = 0.05)
                    && relative_eq!(current_positions.head.yaw.abs(), 0.0, epsilon = 0.05)
                {
                    head_stiffness = context
                        .fall_protection_parameters
                        .ground_impact_head_stiffness;
                }
            }
            _ => {
                head_stiffness = context
                    .fall_protection_parameters
                    .ground_impact_head_stiffness
            }
        }

        let stiffnesses =
            Joints::from_head_and_body(HeadJoints::fill(head_stiffness), BodyJoints::fill(0.0));

        let fall_protection_command = match motion_command.motion {
            Motion::FallProtection {
                direction: FallDirection::Forward,
            } => Some(JointsCommand {
                positions: Joints::from_head_and_body(
                    HeadJoints {
                        yaw: 0.0,
                        pitch: -0.672,
                    },
                    current_positions.into(),
                ),
                stiffnesses,
            }),
            _ => Some(JointsCommand {
                positions: Joints::from_head_and_body(
                    HeadJoints {
                        yaw: 0.0,
                        pitch: 0.5149,
                    },
                    current_positions.into(),
                ),
                stiffnesses,
            }),
        };

        Ok(MainOutputs {
            fall_protection_command,
        })
    }
}
