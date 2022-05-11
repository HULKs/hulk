use anyhow::Context;

use crate::{hardware::HardwareInterface, types::SensorData};

pub fn receive_sensor_data<Hardware>(hardware_interface: &Hardware) -> anyhow::Result<SensorData>
where
    Hardware: HardwareInterface,
{
    hardware_interface
        .produce_sensor_data()
        .context("Failed to produce sensor data")
}
