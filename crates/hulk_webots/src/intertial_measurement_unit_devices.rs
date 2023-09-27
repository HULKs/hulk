use color_eyre::{eyre::WrapErr, Result};
use nalgebra::vector;
use types::sensor_data::InertialMeasurementUnitData;
use webots::{Accelerometer, Gyro, InertialUnit, Robot};

use super::hardware_interface::SIMULATION_TIME_STEP;

pub struct InertialMeasurementUnitDevices {
    accelerometer: Accelerometer,
    gyroscope: Gyro,
    inertial_unit: InertialUnit,
}

impl Default for InertialMeasurementUnitDevices {
    fn default() -> Self {
        let accelerometer = Robot::get_accelerometer("IMU accelerometer");
        accelerometer.enable(SIMULATION_TIME_STEP);

        let gyroscope = Robot::get_gyro("IMU gyro");
        gyroscope.enable(SIMULATION_TIME_STEP);

        let inertial_unit = Robot::get_inertial_unit("IMU inertial");
        inertial_unit.enable(SIMULATION_TIME_STEP);

        Self {
            accelerometer,
            gyroscope,
            inertial_unit,
        }
    }
}

impl InertialMeasurementUnitDevices {
    pub fn get_values(&self) -> Result<InertialMeasurementUnitData> {
        let accelerometer = self
            .accelerometer
            .get_values()
            .wrap_err("failed to get accelerometer values")?;
        let gyroscope = self
            .gyroscope
            .get_values()
            .wrap_err("failed to get gyroscope values")?;
        let inertial_unit = self
            .inertial_unit
            .get_roll_pitch_yaw()
            .wrap_err("failed to get inertial measurement unit values")?;

        Ok(InertialMeasurementUnitData {
            linear_acceleration: vector![
                accelerometer[0] as f32,
                accelerometer[1] as f32,
                accelerometer[2] as f32
            ],
            angular_velocity: vector![
                gyroscope[0] as f32,
                gyroscope[1] as f32,
                gyroscope[2] as f32
            ],
            roll_pitch: vector![inertial_unit[0] as f32, inertial_unit[1] as f32],
        })
    }
}
