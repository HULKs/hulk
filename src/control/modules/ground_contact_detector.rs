use std::time::{Duration, SystemTime, UNIX_EPOCH};

use macros::{module, require_some};

use crate::{
    control::filtering::greater_than_with_hysteresis,
    types::{SensorData, SolePressure},
};

pub struct GroundContactDetector {
    last_has_pressure: bool,
    last_time_switched: SystemTime,
    has_ground_contact: bool,
}

#[module(control)]
#[input(path = sensor_data, data_type = SensorData)]
#[input(path = sole_pressure, data_type = SolePressure)]
#[parameter(path = control.ground_contact_detector.pressure_threshold, data_type = f32)]
#[parameter(path = control.ground_contact_detector.hysteresis, data_type = f32)]
#[parameter(path = control.ground_contact_detector.timeout, data_type = Duration)]
#[main_output(data_type = bool, name = has_ground_contact)]
impl GroundContactDetector {}

impl GroundContactDetector {
    fn new(_context: NewContext) -> anyhow::Result<Self> {
        Ok(Self {
            last_has_pressure: false,
            last_time_switched: UNIX_EPOCH,
            has_ground_contact: false,
        })
    }

    fn cycle(&mut self, context: CycleContext) -> anyhow::Result<MainOutputs> {
        let sensor_data = require_some!(context.sensor_data);
        let sole_pressure = require_some!(context.sole_pressure);
        let has_pressure = greater_than_with_hysteresis(
            self.last_has_pressure,
            sole_pressure.total(),
            *context.pressure_threshold,
            *context.hysteresis,
        );
        self.last_has_pressure = has_pressure;
        if sensor_data
            .cycle_info
            .start_time
            .duration_since(self.last_time_switched)
            .expect("Time ran backwards")
            > *context.timeout
        {
            self.last_time_switched = sensor_data.cycle_info.start_time;
            self.has_ground_contact = has_pressure;
        }
        Ok(MainOutputs {
            has_ground_contact: Some(self.has_ground_contact),
        })
    }
}
