use anyhow::Result;
use approx::relative_eq;
use macros::{module, require_some};

use crate::types::{HeadJoints, HeadMotionSafeExits, HeadMotionType, SensorData};

pub struct ZeroAnglesHead;

#[module(control)]
#[input(path = sensor_data, data_type = SensorData)]
#[persistent_state(path = head_motion_safe_exits, data_type = HeadMotionSafeExits)]
#[main_output(name = zero_angles_head, data_type = HeadJoints)]
impl ZeroAnglesHead {}

impl ZeroAnglesHead {
    fn new(_context: NewContext) -> anyhow::Result<Self> {
        Ok(Self)
    }

    fn cycle(&self, context: CycleContext) -> Result<MainOutputs> {
        let sensor_data = require_some!(context.sensor_data);
        let current_head_angles = sensor_data.positions.head;

        context.head_motion_safe_exits[HeadMotionType::ZeroAngles] =
            relative_eq!(current_head_angles.yaw, 0.0, epsilon = 0.05)
                && relative_eq!(current_head_angles.pitch, 0.0, epsilon = 0.05);

        Ok(MainOutputs {
            zero_angles_head: Some(HeadJoints::default()),
        })
    }
}
