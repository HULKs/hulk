use macros::{module, require_some};

use crate::types::{SensorData, Side, SupportFoot};

pub struct SupportFootEstimation {
    last_support_side: Side,
}

#[module(control)]
#[input(path = sensor_data, data_type = SensorData)]
#[main_output(data_type = SupportFoot)]
impl SupportFootEstimation {}

impl SupportFootEstimation {
    fn new(_context: NewContext) -> anyhow::Result<Self> {
        Ok(Self {
            last_support_side: Side::Left,
        })
    }

    fn cycle(&mut self, context: CycleContext) -> anyhow::Result<MainOutputs> {
        let sensor_data = require_some!(context.sensor_data);

        let left_sum = sensor_data.force_sensitive_resistors.left.sum();
        let right_sum = sensor_data.force_sensitive_resistors.right.sum();

        let support_side = if left_sum > right_sum {
            Side::Left
        } else {
            Side::Right
        };
        let changed_this_cycle = support_side != self.last_support_side;
        self.last_support_side = support_side;

        Ok(MainOutputs {
            support_foot: Some(SupportFoot {
                support_side,
                changed_this_cycle,
            }),
        })
    }
}
