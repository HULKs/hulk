use std::time::{Duration, SystemTime};

use approx::relative_eq;
use macros::{module, require_some};

use crate::{
    framework::configuration::FallProtectionParameters,
    types::{
        FallDirection, FallProtection as FallProtectionData, HeadJoints, HeadMotionType, Motion,
        MotionCommand, MotionSelection, SensorData,
    },
};

pub struct FallProtection {
    start_time: SystemTime,
}

#[module(control)]
#[parameter(name = fall_protection_parameters, path = control.fall_protection_parameters, data_type = FallProtectionParameters)]
#[input(path = sensor_data, data_type = SensorData)]
#[input(path = motion_selection, data_type = MotionSelection)]
#[input(path = motion_command, data_type = MotionCommand)]
#[main_output(data_type = FallProtectionData, name = fall_protection)]

impl FallProtection {}

impl FallProtection {
    fn new(_context: NewContext) -> anyhow::Result<Self> {
        Ok(Self {
            start_time: SystemTime::now(),
        })
    }

    fn cycle(&mut self, context: CycleContext) -> anyhow::Result<MainOutputs> {
        let motion_selection = require_some!(context.motion_selection);
        let motion_command = require_some!(context.motion_command);
        let current_head_angles = require_some!(context.sensor_data).positions.head;

        let mut head_stiffness = 1.0;

        let default_output = Ok(MainOutputs {
            fall_protection: Some(FallProtectionData {
                head_position: current_head_angles,
                head_stiffness,
            }),
        });

        if motion_selection.current_head_motion != HeadMotionType::FallProtection {
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
                if relative_eq!(current_head_angles.pitch, -0.672, epsilon = 0.05)
                    && relative_eq!(current_head_angles.yaw.abs(), 0.0, epsilon = 0.05)
                {
                    head_stiffness = context
                        .fall_protection_parameters
                        .ground_impact_head_stiffness;
                }
            }
            Motion::FallProtection { .. } => {
                if relative_eq!(current_head_angles.pitch, 0.5149, epsilon = 0.05)
                    && relative_eq!(current_head_angles.yaw.abs(), 0.0, epsilon = 0.05)
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

        let fall_protection_data = match motion_command.motion {
            Motion::FallProtection {
                direction: FallDirection::Forward,
            } => Some(FallProtectionData {
                head_position: HeadJoints {
                    yaw: 0.0,
                    pitch: -0.672,
                },
                head_stiffness,
            }),
            _ => Some(FallProtectionData {
                head_position: HeadJoints {
                    yaw: 0.0,
                    pitch: 0.5149,
                },
                head_stiffness,
            }),
        };

        Ok(MainOutputs {
            fall_protection: fall_protection_data,
        })
    }
}
