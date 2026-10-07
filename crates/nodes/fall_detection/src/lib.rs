use std::{future::Future, pin::Pin, sync::Arc, time::Duration};

use booster::{JointsMotorState, LowState};
use color_eyre::{Result, eyre::ensure};
use kinematics::joints::Joints;
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
            self.maximum_stand_up_pose_deviation,
            self.maximum_stand_up_joint_speed,
        ]
        .into_iter()
        .all(|value| value.is_finite() && value > 0.0);
        if !tilts_ordered
            || !thresholds_positive
            || !self.stand_up_pose.into_iter().all(f32::is_finite)
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
}

#[derive(Default)]
pub struct Detector {
    state: Option<State>,
    candidate: Option<(State, Time)>,
    stand_up_pose_since: Option<Time>,
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
        if let Some(last_time) = self.last_time {
            if observation.time <= last_time {
                return None;
            }
            if observation.time.duration_since(last_time) > parameters.maximum_input_age {
                *self = Self::default();
            }
        }
        self.last_time = Some(observation.time);

        if is_in_stand_up_pose(&observation, parameters) {
            self.stand_up_pose_since.get_or_insert(observation.time);
        } else {
            self.stand_up_pose_since = None;
        }
        let is_pose_stable = self.stand_up_pose_since.is_some_and(|since| {
            observation.time.duration_since(since) >= parameters.stand_up_stable_duration
        });
        // Latched while requested: the stand up policy moves the joints out of the pose before
        // the request may be observed here.
        self.ready_for_standup = self.state == Some(State::Fallen)
            && (is_pose_stable || (self.ready_for_standup && stand_up_requested));

        let is_upright = observation.tilt < parameters.upright_tilt;
        let is_lying_still = observation.tilt >= parameters.fallen_tilt
            && observation.angular_speed < parameters.maximum_fallen_angular_speed;
        let transition = match self.state {
            Some(State::Upright) => (observation.tilt >= parameters.falling_tilt)
                .then_some((State::Falling, parameters.falling_duration)),
            None | Some(State::Falling) => {
                if is_upright {
                    Some((State::Upright, parameters.upright_duration))
                } else if is_lying_still {
                    Some((State::Fallen, parameters.fallen_duration))
                } else {
                    None
                }
            }
            Some(State::Fallen) if stand_up_requested && self.ready_for_standup => Some((
                State::StandingUp {
                    since: observation.time,
                },
                Duration::ZERO,
            )),
            Some(State::Fallen) => {
                is_upright.then_some((State::Upright, parameters.upright_duration))
            }
            Some(State::StandingUp { since }) => {
                if observation.time.duration_since(since) >= parameters.stand_up_timeout {
                    Some((State::Fallen, Duration::ZERO))
                } else {
                    is_upright.then_some((State::Upright, parameters.upright_duration))
                }
            }
        };
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
        };
        Some(FallDetection {
            time: observation.time,
            posture,
        })
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

/// Behavior commands Prepare while fallen, driving the joints into this pose. Only then is the
/// starting pose known to be untwisted and within what the stand up policy expects.
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

    let mut detector = Detector::default();
    loop {
        let low_state = low_state_subscriber.recv_with_metadata().await?;
        let Ok(observation) = Observation::from_low_state(&low_state, low_state.source_time) else {
            continue;
        };
        let parameters = parameters.snapshot();
        let parameters = parameters.typed();
        let now = node.clock().now();
        let stand_up_requested =
            motion_command_cache
                .get_latest_with_stamp()
                .is_some_and(|(time, command)| {
                    matches!(*command, MotionCommand::StandUp { .. })
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
            run(&mut detector, 2010, 2200, 0.1, 1.0, true),
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
