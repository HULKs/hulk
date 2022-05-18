use anyhow::Result;
use macros::{module, require_some};
use nalgebra::UnitComplex;

use crate::{
    control::filtering,
    framework::configuration,
    types::{SensorData, SolePressure, SupportFoot},
};

pub struct OrientationFilter {
    orientation_filter: filtering::OrientationFilter,
}

#[module(control)]
#[input(path = sensor_data, data_type = SensorData)]
#[input(path = support_foot, data_type = SupportFoot)]
#[input(path = sole_pressure, data_type = SolePressure)]
#[parameter(path = control.orientation_filter, data_type = configuration::OrientationFilter)]
#[main_output(name = robot_orientation, data_type = UnitComplex<f32>)]
impl OrientationFilter {}

impl OrientationFilter {
    fn new(_context: NewContext) -> anyhow::Result<Self> {
        Ok(Self {
            orientation_filter: filtering::OrientationFilter::default(),
        })
    }

    fn cycle(&mut self, context: CycleContext) -> Result<MainOutputs> {
        let orientation_filter_parameters = context.orientation_filter;
        let sensor_data = require_some!(context.sensor_data);
        let sole_pressure = require_some!(context.sole_pressure);

        // estimate orientation using IMU
        let measured_acceleration = sensor_data.inertial_measurement_unit.linear_acceleration;
        let measured_angular_velocity = sensor_data.inertial_measurement_unit.angular_velocity;
        let cycle_duration = sensor_data.cycle_info.last_cycle_duration;
        self.orientation_filter.update(
            measured_acceleration,
            measured_angular_velocity,
            sole_pressure.left,
            sole_pressure.right,
            cycle_duration.as_secs_f32(),
            orientation_filter_parameters,
        );

        Ok(MainOutputs {
            robot_orientation: Some(self.orientation_filter.yaw()),
        })
    }
}
