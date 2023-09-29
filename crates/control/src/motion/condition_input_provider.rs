use color_eyre::Result;
use context_attribute::context;
use filtering::low_pass_filter::LowPassFilter;
use framework::MainOutput;
use nalgebra::Vector3;
use types::{condition_input::ConditionInput, fall_state::FallState, sensor_data::SensorData};

#[derive(Default)]
pub struct ConditionInputProvider {
    angular_velocity_filter: LowPassFilter<Vector3<f32>>,
}

#[context]
pub struct CreationContext {
    angular_velocity_smoothing_factor: Parameter<f32, "angular_velocity_smoothing_factor">,
}

#[context]
pub struct CycleContext {
    sensor_data: Input<SensorData, "sensor_data">,
    fall_state: Input<FallState, "fall_state">,
}

#[context]
#[derive(Default)]
pub struct MainOutputs {
    pub condition_input: MainOutput<ConditionInput>,
}

impl ConditionInputProvider {
    pub fn new(context: CreationContext) -> Result<Self> {
        Ok(Self {
            angular_velocity_filter: LowPassFilter::with_smoothing_factor(
                Default::default(),
                *context.angular_velocity_smoothing_factor,
            ),
        })
    }

    pub fn cycle(&mut self, context: CycleContext) -> Result<MainOutputs> {
        self.angular_velocity_filter.update(
            context
                .sensor_data
                .inertial_measurement_unit
                .angular_velocity,
        );
        Ok(MainOutputs {
            condition_input: ConditionInput {
                filtered_angular_velocity: self.angular_velocity_filter.state(),
                fall_state: *context.fall_state,
            }
            .into(),
        })
    }
}
