use std::time::Duration;

use anyhow::Result;
use macros::{module, require_some};

use crate::{
    control::linear_interpolator::LinearInterpolator,
    types::{
        DispatchingHeadPositions, HeadJoints, HeadMotionSafeExits, HeadMotionType, MotionSelection,
        SensorData,
    },
};

pub struct DispatchingHeadInterpolator {
    interpolator: LinearInterpolator<HeadJoints>,
    last_currently_active: bool,
    last_dispatching_motion: HeadMotionType,
}

#[module(control)]
#[input(path = sensor_data, data_type = SensorData)]
#[input(path = motion_selection, data_type = MotionSelection)]
#[input(path = look_around, data_type = HeadJoints)]
#[input(path = look_at, data_type = HeadJoints)]
#[input(path = zero_angles_head, data_type = HeadJoints)]
#[parameter(path = control.center_head_position, data_type = HeadJoints)]
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
        let zero_angles_head = require_some!(context.zero_angles_head);
        context.head_motion_safe_exits[HeadMotionType::Dispatching] = false;

        let sensor_data = require_some!(context.sensor_data);
        let motion_selection = require_some!(context.motion_selection);

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
            self.interpolator = match dispatching_head_motion {
                HeadMotionType::Center => LinearInterpolator::new(
                    HeadJoints::from(sensor_data.positions),
                    *context.center_head_position,
                    Duration::from_secs(1),
                ),
                HeadMotionType::Dispatching => panic!("Dispatching cannot dispatch itself"),
                HeadMotionType::LookAround => LinearInterpolator::new(
                    HeadJoints::from(sensor_data.positions),
                    *look_around,
                    Duration::from_secs(1),
                ),
                HeadMotionType::LookAt => LinearInterpolator::new(
                    HeadJoints::from(sensor_data.positions),
                    *look_at,
                    Duration::from_secs(1),
                ),
                HeadMotionType::FallProtection => {
                    panic!("FallProtection shouldn't be interpolated, but executed immediately")
                }

                HeadMotionType::Unstiff => panic!("Dispatching Unstiff is not supported"),
                HeadMotionType::ZeroAngles => LinearInterpolator::new(
                    HeadJoints::from(sensor_data.positions),
                    *zero_angles_head,
                    Duration::from_secs_f32(0.3),
                ),
            };
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
