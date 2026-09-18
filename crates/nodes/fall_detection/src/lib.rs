use std::{future::Future, pin::Pin, sync::Arc, time::Duration};

use booster::{JointsMotorState, LowState};
use color_eyre::{Result, eyre::ensure};
use ros_z::{
    prelude::*,
    qos::{QosHistory, QosReliability},
    time::Time,
};
use serde::{Deserialize, Serialize};
use types::fall_detection::{FALL_DETECTION_TOPIC, FallDetection, Posture};

#[derive(Clone, Debug, Serialize, Deserialize, Message)]
#[serde(deny_unknown_fields)]
pub struct Parameters {
    pub maximum_sensor_age: Duration,
    pub falling_tilt: f32,
    pub fallen_tilt: f32,
    pub upright_tilt: f32,
    pub falling_duration: Duration,
    pub fallen_duration: Duration,
    pub upright_duration: Duration,
    pub maximum_recovery_angular_speed: f32,
    pub maximum_ready_angular_speed: f32,
    pub maximum_ready_joint_speed: f32,
    pub maximum_ready_hip_pitch: f32,
    pub maximum_ready_hip_roll: f32,
    pub maximum_ready_knee: f32,
}

impl Parameters {
    pub fn validate(&self) -> std::result::Result<(), String> {
        if !(0.0 < self.upright_tilt
            && self.upright_tilt < self.falling_tilt
            && self.falling_tilt < self.fallen_tilt
            && self.fallen_tilt < std::f32::consts::PI)
            || [
                self.maximum_sensor_age,
                self.falling_duration,
                self.fallen_duration,
                self.upright_duration,
            ]
            .into_iter()
            .any(|v| v.is_zero())
            || [
                self.maximum_recovery_angular_speed,
                self.maximum_ready_angular_speed,
                self.maximum_ready_joint_speed,
                self.maximum_ready_hip_pitch,
                self.maximum_ready_hip_roll,
                self.maximum_ready_knee,
            ]
            .into_iter()
            .any(|v| !v.is_finite() || v <= 0.0)
        {
            return Err("invalid fall detection thresholds or durations".into());
        }
        Ok(())
    }
}

/// Only measured data is used; no dependence on SDK fall flags or ground transforms.
#[derive(Clone, Copy, Debug)]
pub struct Observation {
    pub time: Time,
    pub tilt: f32,
    pub angular_speed: f32,
    pub leg_speed: f32,
    pub hip_pitch: f32,
    pub hip_roll: f32,
    pub knee: f32,
}

impl Observation {
    pub fn from_low_state(low: &LowState, time: Time) -> Result<Self> {
        let motors = low.serial_motor_states()?;
        let q = motors.positions();
        let dq = motors.velocities();
        let rpy = low.imu_state.roll_pitch_yaw;
        let gyro = low.imu_state.angular_velocity;
        ensure!(
            q.into_iter()
                .chain(dq)
                .chain(rpy.inner.iter().copied())
                .chain(gyro.inner.iter().copied())
                .all(f32::is_finite),
            "invalid fall observation"
        );
        let leg_speed = (dq.into_iter().skip(10).map(|v| v * v).sum::<f32>() / 12.0).sqrt();
        Ok(Self {
            time,
            tilt: (rpy.x().cos() * rpy.y().cos()).clamp(-1.0, 1.0).acos(),
            angular_speed: gyro.inner.norm(),
            leg_speed,
            hip_pitch: q.left_leg.hip_pitch.abs().max(q.right_leg.hip_pitch.abs()),
            hip_roll: q.left_leg.hip_roll.abs().max(q.right_leg.hip_roll.abs()),
            knee: q.left_leg.knee.abs().max(q.right_leg.knee.abs()),
        })
    }
}

#[derive(Default)]
pub struct Detector {
    last: Option<Observation>,
    posture: Posture,
    candidate: Option<(Posture, Time)>,
    ready_since: Option<Time>,
}

impl Detector {
    pub fn invalidate(&mut self) {
        self.last = None;
        self.posture = Posture::Unknown;
        self.candidate = None;
        self.ready_since = None;
    }

    pub fn update(&mut self, observation: Observation, now: Time, p: &Parameters) {
        let finite = [
            observation.tilt,
            observation.angular_speed,
            observation.leg_speed,
            observation.hip_pitch,
            observation.hip_roll,
            observation.knee,
        ]
        .into_iter()
        .all(f32::is_finite);
        if !finite || observation.time > now {
            self.invalidate();
            return;
        }
        if now.duration_since(observation.time) > p.maximum_sensor_age {
            return;
        }
        if let Some(last) = self.last {
            if observation.time <= last.time {
                return;
            }
            if observation.time.duration_since(last.time) > p.maximum_sensor_age {
                self.invalidate();
            }
        }
        self.update_readiness(observation, p);
        self.update_posture(observation, p);
        self.last = Some(observation);
    }

    fn update_readiness(&mut self, observation: Observation, p: &Parameters) {
        let ready = observation.tilt < p.upright_tilt
            && observation.angular_speed < p.maximum_ready_angular_speed
            && observation.leg_speed < p.maximum_ready_joint_speed
            && observation.hip_pitch < p.maximum_ready_hip_pitch
            && observation.hip_roll < p.maximum_ready_hip_roll
            && observation.knee < p.maximum_ready_knee;
        if ready {
            self.ready_since.get_or_insert(observation.time);
        } else {
            self.ready_since = None;
        }
    }

    fn update_posture(&mut self, observation: Observation, p: &Parameters) {
        let next = if observation.tilt >= p.fallen_tilt
            && observation.angular_speed < p.maximum_recovery_angular_speed
        {
            Some((Posture::Fallen, p.fallen_duration))
        } else if observation.tilt >= p.falling_tilt {
            Some((Posture::Falling, p.falling_duration))
        } else if observation.tilt < p.upright_tilt {
            Some((Posture::Upright, p.upright_duration))
        } else {
            None
        };
        if let Some((next, duration)) = next {
            if next == self.posture {
                self.candidate = None;
            } else {
                let since = match self.candidate {
                    Some((candidate, since)) if candidate == next => since,
                    _ => {
                        self.candidate = Some((next, observation.time));
                        observation.time
                    }
                };
                if observation.time.duration_since(since) >= duration {
                    self.posture = next;
                    self.candidate = None;
                }
            }
        } else {
            self.candidate = None;
        }
    }

    pub fn status(&mut self, now: Time, p: &Parameters) -> FallDetection {
        if self
            .last
            .is_some_and(|o| o.time > now || now.duration_since(o.time) > p.maximum_sensor_age)
        {
            self.invalidate();
        }
        FallDetection {
            time: now,
            sample_time: self.last.map_or(now, |o| o.time),
            posture: self.posture,
            tilt: self.last.map_or(0.0, |o| o.tilt),
            angular_speed: self.last.map_or(0.0, |o| o.angular_speed),
            ready_for_walk: self
                .ready_since
                .is_some_and(|t| now >= t && now.duration_since(t) >= p.upright_duration)
                && self.posture == Posture::Upright,
        }
    }
}

pub fn run_boxed(ctx: Arc<Context>) -> Pin<Box<dyn Future<Output = Result<()>> + Send>> {
    Box::pin(run(ctx))
}

async fn run(ctx: Arc<Context>) -> Result<()> {
    let node = ctx.create_node("fall_detection").build().await?;
    let parameters = node.bind_parameter_as::<Parameters>("fall_detection")?;
    parameters.add_validation_hook(Parameters::validate)?;
    let qos = QosProfile {
        reliability: QosReliability::BestEffort,
        history: QosHistory::from_depth(1),
        ..Default::default()
    };
    let sensors = node
        .subscriber::<LowState>("inputs/low_state")
        .qos(qos)
        .build()
        .await?;
    let statuses = node
        .publisher::<FallDetection>(FALL_DETECTION_TOPIC)
        .qos(qos)
        .build()
        .await?;
    let mut detector = Detector::default();
    let mut timer = node.create_timer(Duration::from_millis(10));
    loop {
        tokio::select! {
            received = sensors.recv_with_metadata() => {
                let received = received?;
                match Observation::from_low_state(&received, received.source_time) {
                    Ok(observation) => detector.update(observation, node.clock().now(), parameters.snapshot().typed()),
                    Err(_) => detector.invalidate(),
                }
            }
            _ = timer.tick() => {
                let status = detector.status(node.clock().now(), parameters.snapshot().typed());
                statuses.publish(&status).await?;
            }
        }
    }
}
