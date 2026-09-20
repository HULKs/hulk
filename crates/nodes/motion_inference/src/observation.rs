use ::kinematics::joints::Joints;
use color_eyre::eyre::{Result, ensure};
use coordinate_systems::Robot;
use linear_algebra::Vector3;
use nalgebra::{Quaternion, UnitQuaternion};
use ros_z::time::Time;

use crate::config::Parameters;

#[derive(Clone, Debug)]
pub struct SensorFrame {
    pub timestamp: Time,
    pub position: Joints<f32>,
    pub velocity: Joints<f32>,
    pub orientation: Quaternion<f32>,
    pub gyro: Vector3<Robot>,
    pub last_commanded_position: Joints<f32>,
}

impl SensorFrame {
    pub fn validate_at(&self, now: Time, parameters: &Parameters) -> Result<()> {
        self.validate(parameters)?;
        ensure!(self.timestamp <= now, "sensor timestamp is in the future");
        ensure!(
            now.duration_since(self.timestamp) <= parameters.timing.maximum_sensor_age,
            "sensor frame expired"
        );
        Ok(())
    }

    pub fn validate(&self, parameters: &Parameters) -> Result<()> {
        ensure!(
            self.position
                .into_iter()
                .chain(self.velocity)
                .chain(self.last_commanded_position)
                .chain(self.orientation.coords.iter().copied())
                .chain(self.gyro.inner.iter().copied())
                .all(f32::is_finite),
            "non-finite sensor frame"
        );
        let q = self.orientation;
        let norm = (q.w * q.w + q.i * q.i + q.j * q.j + q.k * q.k).sqrt();
        ensure!(
            (norm - 1.0).abs() < parameters.observation.quaternion_norm_tolerance,
            "invalid orientation quaternion norm: {norm}"
        );
        Ok(())
    }

    pub fn rotation(&self) -> UnitQuaternion<f32> {
        UnitQuaternion::new_normalize(self.orientation)
    }

    pub fn gravity(&self) -> [f32; 3] {
        (self.rotation().inverse() * nalgebra::Vector3::new(0.0, 0.0, -1.0)).into()
    }
}

#[derive(Clone, Default)]
pub struct VelocityEstimator {
    previous: Option<(Time, Joints<f32>)>,
    pub walking: Joints<f32>,
    pub get_up: Joints<f32>,
}

impl VelocityEstimator {
    pub fn update(&mut self, sensor: &SensorFrame, parameters: &Parameters) -> Result<()> {
        if let Some((time, _)) = self.previous {
            ensure!(sensor.timestamp >= time, "sensor time moved backwards");
            if sensor.timestamp == time {
                return Ok(());
            }
        }
        let recent_sample = self.previous.and_then(|(time, position)| {
            let elapsed_frames = (sensor.timestamp.duration_since(time).as_secs_f32()
                / parameters.timing.sensor_period.as_secs_f32())
            .floor()
            .max(1.0);
            (elapsed_frames <= parameters.observation.maximum_velocity_sample_gap_frames)
                .then_some((position, elapsed_frames))
        });
        let maximum_change = parameters
            .observation
            .maximum_walking_velocity_change_degrees
            .to_radians();
        let sample_frequency = (1.0 / parameters.timing.sensor_period.as_secs_f64()) as f32;
        for (joint, position) in sensor.position.enumerate() {
            let velocity = match recent_sample {
                Some((previous_position, elapsed_frames)) => {
                    (position - previous_position[joint]) * sample_frequency / elapsed_frames
                }
                None => sensor.velocity[joint],
            };
            self.get_up[joint] = velocity;
            let previous_velocity = self.walking[joint];
            self.walking[joint] += (velocity - previous_velocity).clamp(
                (-maximum_change).min(-previous_velocity),
                maximum_change.max(-previous_velocity),
            );
        }
        self.previous = Some((sensor.timestamp, sensor.position));
        Ok(())
    }
}
