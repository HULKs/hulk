use std::{future::Future, pin::Pin, sync::Arc, time::Duration};

use booster::{JointsMotorState, LowState};
use color_eyre::{Result, eyre::ensure};
use kinematics::joints::{Joints, leg::LegJoints};
use ros_z::{
    prelude::*,
    qos::{QosHistory, QosReliability},
    time::Time,
};
use serde::{Deserialize, Serialize};
use types::{
    fall_detection::{FallDetection, Posture},
    motion_command::MotionCommand,
};

mod posture;

#[derive(Clone, Debug, Serialize, Deserialize, Message)]
#[serde(deny_unknown_fields)]
pub struct Parameters {
    pub maximum_input_age: Duration,
    pub upright_tilt: f32,
    pub falling_tilt: f32,
    pub fallen_tilt: f32,
    pub upright_duration: Duration,
    pub falling_duration: Duration,
    pub fallen_duration: Duration,
    pub maximum_fallen_angular_speed: f32,
    pub maximum_fallen_joint_speed: f32,
    pub maximum_upright_angular_speed: f32,
    pub maximum_upright_joint_speed: f32,
    pub maximum_upright_leg_deviation: LegJoints<f32>,
    pub minimum_upright_foot_depth: f32,
    pub stand_up_timeout: Duration,
    pub stand_up_pose: Joints<f32>,
    pub maximum_stand_up_pose_deviation: f32,
    pub maximum_stand_up_joint_speed: f32,
    pub stand_up_stable_duration: Duration,
}

impl Parameters {
    pub fn validate(&self) -> std::result::Result<(), String> {
        let tilts_ordered = 0.0 < self.upright_tilt
            && self.upright_tilt < self.falling_tilt
            && self.falling_tilt < self.fallen_tilt
            && self.fallen_tilt < std::f32::consts::PI;
        let thresholds_positive = [
            self.maximum_fallen_angular_speed,
            self.maximum_fallen_joint_speed,
            self.maximum_upright_angular_speed,
            self.maximum_upright_joint_speed,
            self.minimum_upright_foot_depth,
            self.maximum_stand_up_pose_deviation,
            self.maximum_stand_up_joint_speed,
        ]
        .into_iter()
        .chain(self.maximum_upright_leg_deviation)
        .all(|value| value.is_finite() && value > 0.0);
        if !tilts_ordered
            || !thresholds_positive
            || !self.stand_up_pose.into_iter().all(f32::is_finite)
            || [
                self.maximum_input_age,
                self.upright_duration,
                self.falling_duration,
                self.fallen_duration,
                self.stand_up_stable_duration,
            ]
            .into_iter()
            .any(|duration| duration.is_zero())
            || self.stand_up_timeout <= self.upright_duration.max(self.fallen_duration)
        {
            return Err("invalid fall detection thresholds".into());
        }
        Ok(())
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Observation {
    pub time: Time,
    pub tilt: f32,
    pub angular_speed: f32,
    pub positions: Joints<f32>,
    pub velocities: Joints<f32>,
}

impl Observation {
    pub fn from_low_state(low_state: &LowState, time: Time) -> Result<Self> {
        let motors = low_state.serial_motor_states()?;
        let positions = motors.positions();
        let velocities = motors.velocities();
        let roll_pitch_yaw = low_state.imu_state.roll_pitch_yaw;
        let gyro = low_state.imu_state.angular_velocity;
        ensure!(
            positions
                .into_iter()
                .chain(velocities)
                .chain(roll_pitch_yaw.inner.iter().copied())
                .chain(gyro.inner.iter().copied())
                .all(f32::is_finite),
            "non-finite fall detection input"
        );
        Ok(Self {
            time,
            tilt: (roll_pitch_yaw.x().cos() * roll_pitch_yaw.y().cos())
                .clamp(-1.0, 1.0)
                .acos(),
            angular_speed: gyro.inner.norm(),
            positions,
            velocities,
        })
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum State {
    Upright,
    Falling,
    Fallen,
    StandingUp { since: Time },
    RecoveryFailed,
}

#[derive(Default)]
pub struct Detector {
    state: Option<State>,
    candidate: Option<(State, Time)>,
    stand_up_pose_since: Option<Time>,
    lying_still_since: Option<Time>,
    ready_for_standup: bool,
    last_time: Option<Time>,
}

impl Detector {
    /// `stand_up_requested` is whether behavior currently commands a stand up.
    pub fn update(
        &mut self,
        observation: Observation,
        stand_up_requested: bool,
        parameters: &Parameters,
    ) -> Option<FallDetection> {
        if ![observation.tilt, observation.angular_speed]
            .into_iter()
            .chain(observation.positions)
            .chain(observation.velocities)
            .all(f32::is_finite)
        {
            *self = Self::default();
            return None;
        }
        if let Some(last_time) = self.last_time {
            if observation.time <= last_time {
                return None;
            }
            if observation.time.duration_since(last_time) > parameters.maximum_input_age {
                *self = Self::default();
            }
        }
        self.last_time = Some(observation.time);

        self.update_readiness(&observation, stand_up_requested, parameters);
        let transition = self.transition(&observation, stand_up_requested, parameters);
        self.debounce(transition, observation.time);
        if self.state != Some(State::Fallen) {
            self.ready_for_standup = false;
        }
        let posture = match self.state? {
            State::Upright => Posture::Upright,
            State::Falling => Posture::Falling,
            State::Fallen => Posture::Fallen {
                ready_for_standup: self.ready_for_standup,
            },
            State::StandingUp { .. } => Posture::StandingUp,
            State::RecoveryFailed => Posture::RecoveryFailed,
        };
        Some(FallDetection {
            time: observation.time,
            posture,
        })
    }

    fn update_readiness(&mut self, observation: &Observation, requested: bool, p: &Parameters) {
        if is_in_stand_up_pose(observation, p) {
            self.stand_up_pose_since.get_or_insert(observation.time);
        } else {
            self.stand_up_pose_since = None;
        }
        let stable = self.stand_up_pose_since.is_some_and(|since| {
            observation.time.duration_since(since) >= p.stand_up_stable_duration
        });
        // Preserve the accepted start pose while the requested policy moves away from it.
        self.ready_for_standup =
            self.state == Some(State::Fallen) && (stable || (self.ready_for_standup && requested));
        if posture::is_lying_still(observation, p) {
            self.lying_still_since.get_or_insert(observation.time);
        } else {
            self.lying_still_since = None;
        }
    }

    fn transition(
        &self,
        observation: &Observation,
        requested: bool,
        p: &Parameters,
    ) -> Option<(State, Duration)> {
        match self.state {
            Some(State::Upright) => {
                (observation.tilt >= p.falling_tilt).then_some((State::Falling, p.falling_duration))
            }
            Some(State::Fallen) if requested && self.ready_for_standup => Some((
                State::StandingUp {
                    since: observation.time,
                },
                Duration::ZERO,
            )),
            Some(State::StandingUp { since }) => self.stand_up_transition(observation, since, p),
            Some(State::Fallen) => {
                posture::is_upright(observation, p).then_some((State::Upright, p.upright_duration))
            }
            None | Some(State::Falling | State::RecoveryFailed) => {
                if posture::is_upright(observation, p) {
                    Some((State::Upright, p.upright_duration))
                } else if posture::is_lying_still(observation, p) {
                    Some((State::Fallen, p.fallen_duration))
                } else {
                    None
                }
            }
        }
    }

    fn stand_up_transition(
        &self,
        observation: &Observation,
        since: Time,
        p: &Parameters,
    ) -> Option<(State, Duration)> {
        let upright = posture::is_upright(observation, p);
        let upright_confirmed = upright
            && self.candidate.is_some_and(|(state, start)| {
                state == State::Upright
                    && observation.time.duration_since(start) >= p.upright_duration
            });
        if upright_confirmed {
            return Some((State::Upright, Duration::ZERO));
        }
        if observation.time.duration_since(since) >= p.stand_up_timeout {
            let lying_still = self
                .lying_still_since
                .is_some_and(|start| observation.time.duration_since(start) >= p.fallen_duration);
            return Some((
                if lying_still {
                    State::Fallen
                } else {
                    State::RecoveryFailed
                },
                Duration::ZERO,
            ));
        }
        upright.then_some((State::Upright, p.upright_duration))
    }

    fn debounce(&mut self, transition: Option<(State, Duration)>, time: Time) {
        let Some((next, duration)) = transition else {
            self.candidate = None;
            return;
        };
        let since = match self.candidate {
            Some((candidate, since)) if candidate == next => since,
            _ => time,
        };
        if time.duration_since(since) >= duration {
            self.state = Some(next);
            self.candidate = None;
        } else {
            self.candidate = Some((next, since));
        }
    }
}

/// Compare with the configured Prepare reference. This is a pose envelope,
/// not proof of collision clearance or membership in the policy's training set.
fn is_in_stand_up_pose(observation: &Observation, parameters: &Parameters) -> bool {
    let deviation = observation.positions - parameters.stand_up_pose;
    // The head does not affect the stand up.
    deviation
        .into_iter()
        .skip(2)
        .all(|deviation| deviation.abs() < parameters.maximum_stand_up_pose_deviation)
        && observation
            .velocities
            .into_iter()
            .skip(2)
            .all(|velocity| velocity.abs() < parameters.maximum_stand_up_joint_speed)
}

pub fn run_boxed(ctx: Arc<Context>) -> Pin<Box<dyn Future<Output = Result<()>> + Send>> {
    Box::pin(run(ctx))
}

async fn run(ctx: Arc<Context>) -> Result<()> {
    let node = ctx.create_node("fall_detection").build().await?;
    let parameters = node.bind_parameter_as::<Parameters>("fall_detection")?;
    parameters.add_validation_hook(Parameters::validate)?;
    let low_state_subscriber = node
        .subscriber::<LowState>("inputs/low_state")
        // Process the newest sample rather than a backlog.
        .qos(QosProfile {
            reliability: QosReliability::BestEffort,
            history: QosHistory::from_depth(1),
            ..Default::default()
        })
        .build()
        .await?;
    let motion_command_cache = node
        .subscriber::<MotionCommand>("behavior/motion_command")
        .cache(1)
        .build()
        .await?;
    let fall_detection_publisher = node
        .publisher::<FallDetection>("fall_detection/status")
        .build()
        .await?;
    let tilt_publisher = node
        .publisher::<f32>("debug/fall_detection/tilt")
        .build()
        .await?;

    let mut detector = Detector::default();
    loop {
        let low_state = low_state_subscriber.recv_with_metadata().await?;
        let Ok(observation) = Observation::from_low_state(&low_state, low_state.source_time) else {
            detector = Detector::default();
            continue;
        };
        tilt_publisher.publish(&observation.tilt).await?;
        let parameters = parameters.snapshot();
        let parameters = parameters.typed();
        let now = node.clock().now();
        if observation.time > now
            || now.duration_since(observation.time) > parameters.maximum_input_age
        {
            detector = Detector::default();
            continue;
        }
        let stand_up_requested =
            motion_command_cache
                .get_latest_with_stamp()
                .is_some_and(|(time, command)| {
                    matches!(*command, MotionCommand::StandUp)
                        && time <= now
                        && now.duration_since(time) <= parameters.maximum_input_age
                });
        if let Some(fall_detection) = detector.update(observation, stand_up_requested, parameters) {
            fall_detection_publisher.publish(&fall_detection).await?;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parameters() -> Parameters {
        Parameters {
            maximum_input_age: Duration::from_millis(50),
            upright_tilt: 0.35,
            falling_tilt: 0.8,
            fallen_tilt: 1.0,
            upright_duration: Duration::from_millis(100),
            falling_duration: Duration::from_millis(20),
            fallen_duration: Duration::from_millis(150),
            maximum_fallen_angular_speed: 2.5,
            maximum_fallen_joint_speed: 1.0,
            maximum_upright_angular_speed: 1.0,
            maximum_upright_joint_speed: 2.0,
            maximum_upright_leg_deviation: LegJoints::fill(0.5),
            minimum_upright_foot_depth: 0.4,
            stand_up_timeout: Duration::from_secs(8),
            stand_up_pose: Joints::fill(0.0),
            maximum_stand_up_pose_deviation: 0.3,
            maximum_stand_up_joint_speed: 1.0,
            stand_up_stable_duration: Duration::from_millis(150),
        }
    }

    fn observation(milliseconds: i64, tilt: f32, positions: f32) -> Observation {
        Observation {
            time: Time::from_nanos(milliseconds * 1_000_000),
            tilt,
            angular_speed: 0.0,
            positions: Joints::fill(positions),
            velocities: Joints::fill(0.0),
        }
    }

    /// Feeds samples every 10 ms in `[from, to]` and returns the last posture.
    fn run(
        detector: &mut Detector,
        from: i64,
        to: i64,
        tilt: f32,
        positions: f32,
        stand_up_requested: bool,
    ) -> Option<Posture> {
        (from..=to)
            .step_by(10)
            .filter_map(|time| {
                detector.update(
                    observation(time, tilt, positions),
                    stand_up_requested,
                    &parameters(),
                )
            })
            .last()
            .map(|fall_detection| fall_detection.posture)
    }

    #[test]
    fn fall_prepare_and_stand_up() {
        let mut detector = Detector::default();
        assert_eq!(run(&mut detector, 0, 50, 0.0, 0.0, false), None);
        assert_eq!(
            run(&mut detector, 60, 200, 0.0, 0.0, false),
            Some(Posture::Upright)
        );
        assert_eq!(
            run(&mut detector, 210, 240, 0.9, 0.0, false),
            Some(Posture::Falling)
        );
        // Lying twisted: fallen but not ready.
        assert_eq!(
            run(&mut detector, 250, 600, 1.5, 1.0, false),
            Some(Posture::Fallen {
                ready_for_standup: false
            })
        );
        // Prepare reached the pose and settled.
        assert_eq!(
            run(&mut detector, 610, 800, 1.5, 0.0, false),
            Some(Posture::Fallen {
                ready_for_standup: true
            })
        );
        // The stand up policy moving the joints must not leave StandingUp.
        assert_eq!(
            run(&mut detector, 810, 2000, 0.9, 1.0, true),
            Some(Posture::StandingUp)
        );
        assert_eq!(
            run(&mut detector, 2010, 2200, 0.1, 0.0, true),
            Some(Posture::Upright)
        );
    }

    #[test]
    fn stand_up_requires_ready() {
        let mut detector = Detector::default();
        run(&mut detector, 0, 300, 1.5, 1.0, false);
        assert_eq!(
            run(&mut detector, 310, 500, 1.5, 1.0, true),
            Some(Posture::Fallen {
                ready_for_standup: false
            })
        );
    }

    #[test]
    fn stand_up_times_out_to_fallen() {
        let mut detector = Detector::default();
        run(&mut detector, 0, 400, 1.5, 0.0, false);
        assert_eq!(
            run(&mut detector, 410, 420, 1.5, 1.0, true),
            Some(Posture::StandingUp)
        );
        // Still requested, but lying twisted after the timeout: no retry without Prepare.
        assert_eq!(
            run(&mut detector, 430, 8500, 1.5, 1.0, true),
            Some(Posture::Fallen {
                ready_for_standup: false
            })
        );
    }

    #[test]
    fn sensor_gap_resets_state() {
        let mut detector = Detector::default();
        run(&mut detector, 0, 200, 0.0, 0.0, false);
        assert_eq!(run(&mut detector, 400, 400, 0.0, 0.0, false), None);
    }
}
