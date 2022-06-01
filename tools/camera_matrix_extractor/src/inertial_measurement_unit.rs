use anyhow::{anyhow, Context};
use nalgebra::{Vector2, Vector3};
use serde_json::{from_value, Value};

#[derive(Debug)]
pub struct InertialMeasurementUnitData {
    pub linear_acceleration: Vector3<f32>,
    pub angular_velocity: Vector3<f32>,
    pub roll_pitch: Vector2<f32>,
}

impl TryFrom<&Value> for InertialMeasurementUnitData {
    type Error = anyhow::Error;

    fn try_from(replay_frame: &Value) -> anyhow::Result<Self> {
        let imu = replay_frame
            .get("imu")
            .ok_or_else(|| anyhow!("replay_frame.get(\"imu\")"))?;
        let imu_accelerometer = imu
            .get("accelerometer")
            .ok_or_else(|| anyhow!("imu.get(\"accelerometer\")"))?;
        let imu_gyroscope = imu
            .get("gyroscope")
            .ok_or_else(|| anyhow!("imu.get(\"gyroscope\")"))?;
        let imu_angle = imu
            .get("angle")
            .ok_or_else(|| anyhow!("imu.get(\"angle\")"))?;
        Ok(Self {
            linear_acceleration: from_value(imu_accelerometer.clone())
                .context("from_value(imu_accelerometer)")?,
            angular_velocity: from_value(imu_gyroscope.clone())
                .context("from_value(imu_gyroscope)")?,
            roll_pitch: from_value(imu_angle.clone()).context("from_value(imu_angle)")?,
        })
    }
}
