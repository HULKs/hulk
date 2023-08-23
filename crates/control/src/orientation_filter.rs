use color_eyre::Result;
use context_attribute::context;
use filtering::orientation_filtering::OrientationFiltering;
use framework::MainOutput;
use nalgebra::UnitComplex;
use types::{
    orientation_filter::{Parameters, State},
    CycleTime, SensorData, SolePressure, SupportFoot,
};

#[derive(Default)]
pub struct OrientationFilter {
    state: State,
}

#[context]
pub struct CreationContext {}

#[context]
pub struct CycleContext {
    sensor_data: Input<SensorData, "sensor_data">,
    cycle_time: Input<CycleTime, "cycle_time">,
    sole_pressure: Input<SolePressure, "sole_pressure">,
    support_foot: Input<SupportFoot, "support_foot">,

    orientation_filter_parameters: Parameter<Parameters, "orientation_filter">,
}

#[context]
#[derive(Default)]
pub struct MainOutputs {
    pub robot_orientation: MainOutput<UnitComplex<f32>>,
}

impl OrientationFilter {
    pub fn new(_context: CreationContext) -> Result<Self> {
        Ok(Default::default())
    }

    pub fn cycle(&mut self, context: CycleContext) -> Result<MainOutputs> {
        let measured_acceleration = context
            .sensor_data
            .inertial_measurement_unit
            .linear_acceleration;
        let measured_angular_velocity = context
            .sensor_data
            .inertial_measurement_unit
            .angular_velocity;
        let cycle_duration = context.cycle_time.last_cycle_duration;
        self.state.update(
            measured_acceleration,
            measured_angular_velocity,
            context.sole_pressure.left,
            context.sole_pressure.right,
            cycle_duration.as_secs_f32(),
            context.orientation_filter_parameters,
        );

        Ok(MainOutputs {
            robot_orientation: self.state.yaw().into(),
        })
    }
}
