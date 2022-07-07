use anyhow::Context;
use types::SensorData;

use crate::hardware::HardwareInterface;

pub fn receive_sensor_data<Hardware>(hardware_interface: &Hardware) -> anyhow::Result<SensorData>
where
    Hardware: HardwareInterface,
{
    hardware_interface
        .produce_sensor_data()
        .context("Failed to produce sensor data")
}
