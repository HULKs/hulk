use std::f32::consts::PI;

use booster::{JointsMotorState, MotorState};
use color_eyre::Result;
use context_attribute::context;
use filtering::low_pass_filter::LowPassFilter;
use framework::MainOutput;
use serde::{Deserialize, Serialize};
use types::{
    cycle_time::CycleTime,
    joints::{head::HeadJoints, Joints},
    motion_command::{HeadMotion as HeadMotionCommand, ImageRegion, MotionCommand},
    parameters::HeadMotionParameters,
};

#[derive(Default, Deserialize, Serialize)]
pub struct HeadMotion {
    last_positions: HeadJoints<f32>,
    lowpass_filter: LowPassFilter<HeadJoints<f32>>,
}

#[context]
pub struct CreationContext {}

#[context]
pub struct CycleContext {
    parameters: Parameter<HeadMotionParameters, "head_motion">,
    // look_around: Input<HeadJoints<f32>, "look_around">,
    look_at: Input<HeadJoints<f32>, "look_at">,
    motor_states: Input<Joints<MotorState>, "serial_motor_states">,
    cycle_time: Input<CycleTime, "cycle_time">,
    motion_command: Input<MotionCommand, "WorldState", "motion_command">,
}

#[context]
#[derive(Default)]
pub struct MainOutputs {
    pub head_joints_command: MainOutput<HeadJoints<f32>>,
}

impl HeadMotion {
    pub fn new(_context: CreationContext) -> Result<Self> {
        Ok(Self {
            last_positions: Default::default(),
            lowpass_filter: LowPassFilter::with_smoothing_factor(Default::default(), 0.075),
        })
    }

    pub fn cycle(&mut self, context: CycleContext) -> Result<MainOutputs> {
        if let Some(injected_head_joints) = context.parameters.injected_head_joints {
            self.lowpass_filter.update(injected_head_joints);

            return Ok(MainOutputs {
                head_joints_command: self.lowpass_filter.state().into(),
            });
        }
        let raw_positions = Self::joints_from_motion(&context);
        let maximum_movement = context.parameters.maximum_velocity
            * context.cycle_time.last_cycle_duration.as_secs_f32();

        let controlled_positions = HeadJoints {
            yaw: self.last_positions.yaw
                + (raw_positions.yaw - self.last_positions.yaw)
                    .clamp(-maximum_movement.yaw, maximum_movement.yaw),
            pitch: self.last_positions.pitch
                + (raw_positions.pitch - self.last_positions.pitch)
                    .clamp(-maximum_movement.pitch, maximum_movement.pitch),
        };

        let clamped_pitch = compute_clamped_pitch(controlled_positions, context.parameters);

        let clamped_positions = HeadJoints {
            pitch: clamped_pitch,
            yaw: controlled_positions.yaw,
        };

        self.last_positions = clamped_positions;
        Ok(MainOutputs {
            head_joints_command: clamped_positions.into(),
        })
    }

    pub fn joints_from_motion(context: &CycleContext) -> HeadJoints<f32> {
        match context.motion_command.head_motion() {
            Some(HeadMotionCommand::Center {
                image_region_target: ImageRegion::Top,
            }) => HeadJoints {
                yaw: 0.0,
                pitch: 0.4,
            },
            Some(HeadMotionCommand::Center { .. }) => HeadJoints {
                yaw: 0.0,
                pitch: 0.4,
            },
            Some(HeadMotionCommand::LookAt { .. })
            | Some(HeadMotionCommand::LookAtReferee { .. })
            | Some(HeadMotionCommand::LookLeftAndRightOf { .. }) => *context.look_at,
            Some(HeadMotionCommand::Unstiff) => context.motor_states.positions().head,
            Some(HeadMotionCommand::Animation { stiff: false }) => {
                context.motor_states.positions().head
            }
            Some(HeadMotionCommand::Animation { stiff: true }) => {
                context.motor_states.positions().head
            }
            Some(_) | None => Default::default(),
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
