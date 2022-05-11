use std::time::Duration;

use anyhow::Result;
use macros::{module, require_some};

use crate::{
    control::linear_interpolator::LinearInterpolator,
    types::{
        BodyJoints, BodyMotionSafeExits, BodyMotionType, DispatchingBodyPositions, Joints,
        MotionSelection, SensorData, SitDownPositions, StandUpBackPositions, StandUpFrontPositions,
        WalkPositions,
    },
};

pub struct DispatchingBodyInterpolator {
    interpolator: LinearInterpolator<BodyJoints>,
    last_currently_active: bool,
    last_dispatching_motion: BodyMotionType,
}

#[module(control)]
#[input(path = sensor_data, data_type = SensorData)]
#[input(path = motion_selection, data_type = MotionSelection)]
#[input(path = stand_up_back_positions, data_type = StandUpBackPositions)]
#[input(path = sit_down_positions, data_type = SitDownPositions)]
#[input(path = stand_up_front_positions, data_type = StandUpFrontPositions)]
#[input(path = walk_positions, data_type = WalkPositions)]
#[persistent_state(path = body_motion_safe_exits, data_type = BodyMotionSafeExits)]
#[parameter(path = control.penalized_pose, data_type = Joints)]
#[parameter(path = control.ready_pose, data_type = Joints)]
#[main_output(data_type = DispatchingBodyPositions)]
impl DispatchingBodyInterpolator {}

impl DispatchingBodyInterpolator {
    pub fn new() -> Self {
        Self {
            interpolator: Default::default(),
            last_currently_active: false,
            last_dispatching_motion: BodyMotionType::Unstiff,
        }
    }

    fn cycle(&mut self, context: CycleContext) -> Result<MainOutputs> {
        context.body_motion_safe_exits[BodyMotionType::Dispatching] = false;

        let sensor_data = require_some!(context.sensor_data);
        let motion_selection = require_some!(context.motion_selection);

        let currently_active = motion_selection.current_body_motion == BodyMotionType::Dispatching;
        if !currently_active {
            self.last_currently_active = currently_active;
            return Ok(MainOutputs {
                dispatching_body_positions: Some(DispatchingBodyPositions {
                    positions: Default::default(),
                }),
            });
        }

        let dispatching_body_motion = require_some!(motion_selection.dispatching_body_motion);
        let stand_up_back_positions = require_some!(context.stand_up_back_positions).positions;
        let stand_up_front_positions = require_some!(context.stand_up_front_positions).positions;
        let walk_positions = require_some!(context.walk_positions).positions;
        let sit_down_positions = require_some!(context.sit_down_positions).positions;

        let interpolator_reset_required = self.last_dispatching_motion != dispatching_body_motion
            || (!self.last_currently_active && currently_active);
        self.last_dispatching_motion = dispatching_body_motion;
        self.last_currently_active = currently_active;

        if interpolator_reset_required {
            self.interpolator = match dispatching_body_motion {
                BodyMotionType::Dispatching => panic!("Dispatching motion cannot be Dispatching"),
                BodyMotionType::FallProtection => {
                    panic!("FallProtection shouldn't be interpolated, but executed immediately")
                }
                BodyMotionType::Jump => todo!(),
                BodyMotionType::Kick => todo!(),
                BodyMotionType::Penalized => LinearInterpolator::new(
                    BodyJoints::from(sensor_data.positions),
                    BodyJoints::from(*context.penalized_pose),
                    Duration::from_secs(1),
                ),
                BodyMotionType::SitDown => LinearInterpolator::new(
                    BodyJoints::from(sensor_data.positions),
                    sit_down_positions,
                    Duration::from_secs(1),
                ),
                BodyMotionType::Stand => LinearInterpolator::new(
                    BodyJoints::from(sensor_data.positions),
                    BodyJoints::from(*context.ready_pose),
                    Duration::from_secs(1),
                ),
                BodyMotionType::StandUpBack => LinearInterpolator::new(
                    BodyJoints::from(sensor_data.positions),
                    stand_up_back_positions,
                    Duration::from_secs(1),
                ),
                BodyMotionType::StandUpFront => LinearInterpolator::new(
                    BodyJoints::from(sensor_data.positions),
                    stand_up_front_positions,
                    Duration::from_secs(1),
                ),
                BodyMotionType::Unstiff => {
                    panic!("Unstiffing shouln't be interpolated, but executed immediately")
                }
                BodyMotionType::Walk => LinearInterpolator::new(
                    BodyJoints::from(sensor_data.positions),
                    walk_positions,
                    Duration::from_secs(1),
                ),
            };
        }

        self.interpolator
            .step(sensor_data.cycle_info.last_cycle_duration);

        context.body_motion_safe_exits[BodyMotionType::Dispatching] =
            self.interpolator.is_finished();

        Ok(MainOutputs {
            dispatching_body_positions: Some(DispatchingBodyPositions {
                positions: self.interpolator.value(),
            }),
        })
    }
}
