use color_eyre::Result;
use serde::{Deserialize, Serialize};

use context_attribute::context;
use coordinate_systems::Field;
use filtering::orientation_filtering::OrientationFiltering;
use framework::MainOutput;
use linear_algebra::Orientation2;
use types::{
    cycle_time::CycleTime,
    orientation_filter::{Parameters, State},
    sensor_data::SensorData,
    sole_pressure::SolePressure,
};

#[derive(Default, Deserialize, Serialize)]
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

    orientation_filter_parameters: Parameter<Parameters, "orientation_filter">,
}

#[context]
#[derive(Default)]
pub struct MainOutputs {
    pub robot_orientation: MainOutput<Orientation2<Field>>,
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
            robot_orientation: Orientation2::wrap(self.state.yaw()).into(),
        })
    }
}
