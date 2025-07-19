use std::f32::consts::PI;

use color_eyre::Result;
use context_attribute::context;
use filtering::low_pass_filter::LowPassFilter;
use framework::MainOutput;
use serde::{Deserialize, Serialize};
use types::{
    cycle_time::CycleTime,
    joints::head::HeadJoints,
    motion_command::{HeadMotion as HeadMotionCommand, MotionCommand},
    motion_selection::MotionSelection,
    motor_commands::MotorCommands,
    parameters::HeadMotionParameters,
    roles::Role,
    sensor_data::SensorData,
};

#[derive(Default, Deserialize, Serialize)]
pub struct HeadMotion {
    last_request: MotorCommands<HeadJoints<f32>>,
    position_filter: LowPassFilter<HeadJoints<f32>>,
}

#[context]
pub struct CreationContext {
    parameters: Parameter<HeadMotionParameters, "head_motion">,
}

#[context]
pub struct CycleContext {
    parameters: Parameter<HeadMotionParameters, "head_motion">,
    center_head_position: Parameter<HeadJoints<f32>, "center_head_position">,

    look_around: Input<HeadJoints<f32>, "look_around">,
    look_at: Input<HeadJoints<f32>, "look_at">,
    motion_command: Input<MotionCommand, "motion_command">,
    sensor_data: Input<SensorData, "sensor_data">,
    cycle_time: Input<CycleTime, "cycle_time">,
    has_ground_contact: Input<bool, "has_ground_contact">,
    motion_selection: Input<MotionSelection, "motion_selection">,
    role: Input<Role, "role">,
}

#[context]
#[derive(Default)]
pub struct MainOutputs {
    pub head_joints_command: MainOutput<MotorCommands<HeadJoints<f32>>>,
}

impl HeadMotion {
    pub fn new(context: CreationContext) -> Result<Self> {
        Ok(Self {
            last_request: Default::default(),
            position_filter: LowPassFilter::with_smoothing_factor(
                Default::default(),
                context.parameters.filter_smoothing_factor,
            ),
        })
    }

    pub fn cycle(&mut self, context: CycleContext) -> Result<MainOutputs> {
        let request = Self::requested_head_motion(&context).unwrap_or(self.last_request);

        let maximum_movement = match context.role {
            Role::DefenderLeft | Role::DefenderRight => {
                context.parameters.maximum_defender_velocity
                    * context.cycle_time.last_cycle_duration.as_secs_f32()
            }
            _ => {
                context.parameters.maximum_velocity
                    * context.cycle_time.last_cycle_duration.as_secs_f32()
            }
        };

        let stiffnesses = request.stiffnesses;
        let positions = request.positions;
        let last_positions = self.last_request.positions;

        let velocity_limited_positions = HeadJoints {
            yaw: last_positions.yaw
                + (positions.yaw - last_positions.yaw)
                    .clamp(-maximum_movement.yaw, maximum_movement.yaw),
            pitch: last_positions.pitch
                + (positions.pitch - last_positions.pitch)
                    .clamp(-maximum_movement.pitch, maximum_movement.pitch),
        };

        self.position_filter.update(velocity_limited_positions);
        let smoothed_positions = self.position_filter.state();
        let smoothed_request = MotorCommands {
            positions: smoothed_positions,
            stiffnesses,
        };
        self.last_request = smoothed_request;

        Ok(MainOutputs {
            head_joints_command: smoothed_request.into(),
        })
    }

    pub fn requested_head_motion(context: &CycleContext) -> Option<MotorCommands<HeadJoints<f32>>> {
        let stiffnesses = HeadJoints::fill(0.8);
        if let Some(positions) = context.parameters.injected_head_joints {
            return Some(MotorCommands {
                positions,
                stiffnesses,
            });
        }

        if context.motion_selection.dispatching_motion.is_some() {
            return None;
        }

        let request = if *context.has_ground_contact {
            Self::joints_from_motion(context)
        } else {
            MotorCommands {
                positions: HeadJoints::default(),
                stiffnesses,
            }
        };

        let clamped_pitch = compute_clamped_pitch(request.positions, context.parameters);

        Some(MotorCommands {
            positions: HeadJoints {
                yaw: request.positions.yaw,
                pitch: clamped_pitch,
            },
            stiffnesses,
        })
    }

    pub fn joints_from_motion(context: &CycleContext) -> MotorCommands<HeadJoints<f32>> {
        let stiffnesses = HeadJoints::fill(0.8);
        match context.motion_command.head_motion() {
            Some(HeadMotionCommand::Center) => MotorCommands {
                positions: *context.center_head_position,
                stiffnesses,
            },
            Some(HeadMotionCommand::LookAround | HeadMotionCommand::SearchForLostBall) => {
                MotorCommands {
                    positions: *context.look_around,
                    stiffnesses,
                }
            }
            Some(HeadMotionCommand::LookAt { .. })
            | Some(HeadMotionCommand::LookAtReferee { .. })
            | Some(HeadMotionCommand::LookLeftAndRightOf { .. }) => MotorCommands {
                positions: *context.look_at,
                stiffnesses,
            },
            Some(HeadMotionCommand::Unstiff) => MotorCommands {
                positions: context.sensor_data.positions.head,
                stiffnesses: HeadJoints::fill(0.0),
            },
            Some(HeadMotionCommand::Animation { stiff: false }) => MotorCommands {
                positions: context.sensor_data.positions.head,
                stiffnesses: HeadJoints::fill(0.0),
            },
            Some(HeadMotionCommand::Animation { stiff: true }) => MotorCommands {
                positions: context.sensor_data.positions.head,
                stiffnesses: HeadJoints::fill(1.0),
            },
            Some(HeadMotionCommand::ZeroAngles) | None => MotorCommands {
                positions: Default::default(),
                stiffnesses,
            },
        }
    }
}

fn compute_clamped_pitch(
    controlled_positions: HeadJoints<f32>,
    head_motion_parameters: &HeadMotionParameters,
) -> f32 {
    let maximum_pitch = if controlled_positions.yaw.abs() >= head_motion_parameters.outer_yaw {
        head_motion_parameters.outer_maximum_pitch
    } else {
        let interpolation_factor =
            0.5 * (1.0 + (PI * controlled_positions.yaw / head_motion_parameters.outer_yaw).cos());
        head_motion_parameters.outer_maximum_pitch
            + interpolation_factor
                * (head_motion_parameters.inner_maximum_pitch
                    - head_motion_parameters.outer_maximum_pitch)
    };

    let minimum_pitch = if controlled_positions.yaw.abs() >= head_motion_parameters.outer_yaw {
        head_motion_parameters.outer_minimum_pitch
    } else {
        let interpolation_factor =
            0.5 * (1.0 + (PI * controlled_positions.yaw / head_motion_parameters.outer_yaw).cos());
        head_motion_parameters.outer_minimum_pitch
            + interpolation_factor
                * (head_motion_parameters.inner_minimum_pitch
                    - head_motion_parameters.outer_minimum_pitch)
    };

    let clamped_maximum_pitch = maximum_pitch.max(0.0);
    let clamped_minimum_pitch = minimum_pitch.min(0.0);

    controlled_positions
        .pitch
        .clamp(clamped_minimum_pitch, clamped_maximum_pitch)
}

#[cfg(test)]
mod test {
    use proptest::prelude::*;

    use super::*;

    proptest! {
        #[test]
        fn clamp_clamping_should_not_panic(yaw in -PI..PI) {
            let head_motion_parameters = HeadMotionParameters {
                inner_maximum_pitch: 0.61,
                inner_minimum_pitch: -0.61,
                outer_maximum_pitch: 0.0,
                outer_minimum_pitch: 0.0,
                outer_yaw: 1.3,
                ..Default::default()
            };

            let controlled_positions = HeadJoints { yaw, ..Default::default() };

            let clamped_pitch = compute_clamped_pitch(controlled_positions, &head_motion_parameters);

            assert!(clamped_pitch <= head_motion_parameters.inner_maximum_pitch);
            assert!(clamped_pitch >= head_motion_parameters.inner_minimum_pitch);
        }
    }
}
