use std::time::Duration;

use anyhow::Result;
use macros::{module, require_some};

use crate::{
    control::linear_interpolator::LinearInterpolator,
    types::{
        DispatchingHeadPositions, HeadJoints, HeadMotionSafeExits, HeadMotionType, MotionSelection,
        SensorData, StandUpBackPositions, StandUpFrontPositions,
    },
};

pub struct DispatchingHeadInterpolator {
    interpolator: LinearInterpolator<HeadJoints>,
    last_currently_active: bool,
    last_dispatching_motion: HeadMotionType,
}

#[module(control)]
#[input(path = look_around, data_type = HeadJoints)]
#[input(path = look_at, data_type = HeadJoints)]
#[input(path = motion_selection, data_type = MotionSelection)]
#[input(path = sensor_data, data_type = SensorData)]
#[input(path = stand_up_back_positions, data_type = StandUpBackPositions)]
#[input(path = stand_up_front_positions, data_type = StandUpFrontPositions)]
#[input(path = zero_angles_head, data_type = HeadJoints)]
#[parameter(path = control.center_head_position, data_type = HeadJoints)]
#[parameter(path = control.dispatching_head_interpolator.maximum_yaw_velocity,  data_type = f32)]
#[parameter(path = control.dispatching_head_interpolator.maximum_pitch_velocity ,data_type = f32)]
#[persistent_state(path = head_motion_safe_exits, data_type = HeadMotionSafeExits)]
#[main_output(data_type = DispatchingHeadPositions)]
impl DispatchingHeadInterpolator {}

impl DispatchingHeadInterpolator {
    fn new(_context: NewContext) -> anyhow::Result<Self> {
        Ok(Self {
            interpolator: Default::default(),
            last_currently_active: false,
            last_dispatching_motion: HeadMotionType::Unstiff,
        })
    }

    fn cycle(&mut self, context: CycleContext) -> Result<MainOutputs> {
        let look_around = require_some!(context.look_around);
        let look_at = require_some!(context.look_at);
        let motion_selection = require_some!(context.motion_selection);
        let sensor_data = require_some!(context.sensor_data);
        let stand_up_back_positions = require_some!(context.stand_up_back_positions);
        let stand_up_front_positions = require_some!(context.stand_up_front_positions);
        let zero_angles_head = require_some!(context.zero_angles_head);

        context.head_motion_safe_exits[HeadMotionType::Dispatching] = false;

        let currently_active = motion_selection.current_head_motion == HeadMotionType::Dispatching;
        if !currently_active {
            return Ok(MainOutputs {
                dispatching_head_positions: Some(DispatchingHeadPositions {
                    positions: Default::default(),
                }),
            });
        }

        let dispatching_head_motion = require_some!(motion_selection.dispatching_head_motion);

        let interpolator_reset_required = self.last_dispatching_motion != dispatching_head_motion
            || (!self.last_currently_active && currently_active);
        self.last_dispatching_motion = dispatching_head_motion;
        self.last_currently_active = currently_active;

        if interpolator_reset_required {
            let start_position = HeadJoints::from(sensor_data.positions);
            let target_position = match dispatching_head_motion {
                HeadMotionType::Center => *context.center_head_position,
                HeadMotionType::Dispatching => panic!("Dispatching cannot dispatch itself"),
                HeadMotionType::FallProtection => panic!("Is executed immediately"),
                HeadMotionType::LookAround => *look_around,
                HeadMotionType::LookAt => *look_at,
                HeadMotionType::StandUpBack => stand_up_back_positions.head_positions,
                HeadMotionType::StandUpFront => stand_up_front_positions.head_positions,
                HeadMotionType::Unstiff => panic!("Dispatching Unstiff doesn't make sense"),
                HeadMotionType::ZeroAngles => *zero_angles_head,
            };
            let duration = time_required_for_transition(
                start_position,
                target_position,
                *context.maximum_yaw_velocity,
                *context.maximum_pitch_velocity,
            );
            self.interpolator = LinearInterpolator::new(start_position, target_position, duration);
        }

        self.interpolator
            .step(sensor_data.cycle_info.last_cycle_duration);

        context.head_motion_safe_exits[HeadMotionType::Dispatching] =
            self.interpolator.is_finished();

        Ok(MainOutputs {
            dispatching_head_positions: Some(DispatchingHeadPositions {
                positions: self.interpolator.value(),
            }),
        })
    }
}

fn time_required_for_transition(
    current_position: HeadJoints,
    target_position: HeadJoints,
    maximum_yaw_velocity: f32,
    maximum_pitch_velocity: f32,
) -> Duration {
    let pitch_time =
        (current_position.pitch - target_position.pitch).abs() / maximum_pitch_velocity;
    let yaw_time = (current_position.yaw - target_position.yaw).abs() / maximum_yaw_velocity;

    Duration::from_secs_f32(f32::max(pitch_time, yaw_time))
}
