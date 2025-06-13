use color_eyre::Result;
use context_attribute::context;
use filtering::hysteresis::greater_than_with_hysteresis;
use framework::{AdditionalOutput, MainOutput};
use serde::{Deserialize, Serialize};
use types::{
    sensor_data::SensorData,
    support_foot::{Side, SupportFoot},
};

#[derive(Deserialize, Serialize)]
pub struct SupportFootEstimation {
    last_support_side: Side,
}

#[context]
pub struct CreationContext {}

#[context]
pub struct CycleContext {
    hysteresis: Parameter<f32, "support_foot_estimation.hysteresis">,

    has_ground_contact: Input<bool, "has_ground_contact">,
    sensor_data: Input<SensorData, "sensor_data">,

    left_sum: AdditionalOutput<f32, "force_sensitive_resistors_left_sum">,
    right_sum: AdditionalOutput<f32, "force_sensitive_resistors_right_sum">,
}

#[context]
#[derive(Default)]
pub struct MainOutputs {
    pub support_foot: MainOutput<SupportFoot>,
}

impl SupportFootEstimation {
    pub fn new(_context: CreationContext) -> Result<Self> {
        Ok(Self {
            last_support_side: Side::Left,
        })
    }

    pub fn cycle(&mut self, mut context: CycleContext) -> Result<MainOutputs> {
        if !context.has_ground_contact {
            return Ok(MainOutputs {
                support_foot: SupportFoot {
                    support_side: None,
                    changed_this_cycle: false,
                }
                .into(),
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

        context.left_sum.fill_if_subscribed(|| left_sum.clone());
        context.right_sum.fill_if_subscribed(|| right_sum.clone());

        Ok(MainOutputs {
            support_foot: SupportFoot {
                support_side: Some(support_side),
                changed_this_cycle,
            }
            .into(),
        })
    }
}
