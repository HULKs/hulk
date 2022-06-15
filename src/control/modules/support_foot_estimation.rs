use macros::{module, require_some};

use crate::{
    control::filtering::Hysteresis,
    types::{GroundContact, SensorData, Side, SupportFoot},
};

pub struct SupportFootEstimation {
    last_support_side: Side,
    hysteresis: Hysteresis,
}

#[module(control)]
#[input(path = sensor_data, data_type = SensorData)]
#[input(path = ground_contact, data_type = GroundContact)]
#[parameter(path = control.support_foot_estimation.hysteresis, data_type = f32)]
#[main_output(data_type = SupportFoot)]
impl SupportFootEstimation {}

impl SupportFootEstimation {
    fn new(_context: NewContext) -> anyhow::Result<Self> {
        Ok(Self {
            last_support_side: Side::Left,
            hysteresis: Hysteresis::new(),
        })
    }

    fn cycle(&mut self, context: CycleContext) -> anyhow::Result<MainOutputs> {
        let sensor_data = require_some!(context.sensor_data);
        let ground_contact = require_some!(context.ground_contact);

        if !ground_contact.any_foot() {
            return Ok(MainOutputs {
                support_foot: Some(SupportFoot {
                    support_side: None,
                    changed_this_cycle: false,
                }),
            });
        }

        let left_sum = sensor_data.force_sensitive_resistors.left.sum();
        let right_sum = sensor_data.force_sensitive_resistors.right.sum();

        let left_has_more_pressure =
            self.hysteresis
                .update_greater_than(left_sum, right_sum, *context.hysteresis);
        let support_side = if left_has_more_pressure {
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
