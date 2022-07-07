use module_derive::module;
use types::{SensorData, Side, SupportFoot};

use crate::control::filtering::greater_than_with_hysteresis;

pub struct SupportFootEstimation {
    last_support_side: Side,
}

#[module(control)]
#[input(path = sensor_data, data_type = SensorData, required)]
#[input(path = has_ground_contact, data_type = bool, required)]
#[parameter(path = control.support_foot_estimation.hysteresis, data_type = f32)]
#[main_output(data_type = SupportFoot)]
impl SupportFootEstimation {}

impl SupportFootEstimation {
    fn new(_context: NewContext) -> anyhow::Result<Self> {
        Ok(Self {
            last_support_side: Side::Left,
        })
    }

    fn cycle(&mut self, context: CycleContext) -> anyhow::Result<MainOutputs> {
        if !context.has_ground_contact {
            return Ok(MainOutputs {
                support_foot: Some(SupportFoot {
                    support_side: None,
                    changed_this_cycle: false,
                }),
            });
        }

        let left_sum = context.sensor_data.force_sensitive_resistors.left.sum();
        let right_sum = context.sensor_data.force_sensitive_resistors.right.sum();

        let last_has_left_more_pressure = self.last_support_side == Side::Left;
        let has_left_more_pressure = greater_than_with_hysteresis(
            last_has_left_more_pressure,
            left_sum,
            right_sum,
            *context.hysteresis,
        );
        let support_side = if has_left_more_pressure {
            Side::Left
        } else {
            Side::Right
        };
        let changed_this_cycle = support_side != self.last_support_side;
        self.last_support_side = support_side;

        Ok(MainOutputs {
            support_foot: Some(SupportFoot {
                support_side: Some(support_side),
                changed_this_cycle,
            }),
        })
    }
}
