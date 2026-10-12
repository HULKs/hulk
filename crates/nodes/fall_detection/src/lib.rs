use std::{f32::consts::PI, future::Future, pin::Pin, sync::Arc, time::Duration};

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
    fall_detection::{FallDetection, MAXIMUM_FALL_DETECTION_AGE, Posture},
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
    pub minimum_upright_foot_depth: f32,
    pub stand_up_timeout: Duration,
    pub stand_up_pose: Joints<f32>,
    pub maximum_stand_up_pose_deviation: f32,
    pub maximum_stand_up_joint_speed: f32,
    pub stand_up_stable_duration: Duration,
}

impl Parameters {
    pub fn validate(&self) -> std::result::Result<(), String> {
        if !(0.0 < self.upright_tilt
            && self.upright_tilt < self.falling_tilt
            && self.falling_tilt < self.fallen_tilt
            && self.fallen_tilt < PI)
        {
            return Err("tilts must satisfy 0 < upright < falling < fallen < pi".into());
        }
        let thresholds = [
            (
                "maximum_fallen_angular_speed",
                self.maximum_fallen_angular_speed,
            ),
            (
                "maximum_fallen_joint_speed",
                self.maximum_fallen_joint_speed,
            ),
            (
                "maximum_upright_angular_speed",
                self.maximum_upright_angular_speed,
            ),
            (
                "maximum_upright_joint_speed",
                self.maximum_upright_joint_speed,
            ),
            (
                "minimum_upright_foot_depth",
                self.minimum_upright_foot_depth,
            ),
            (
                "maximum_stand_up_pose_deviation",
                self.maximum_stand_up_pose_deviation,
            ),
            (
                "maximum_stand_up_joint_speed",
                self.maximum_stand_up_joint_speed,
            ),
        ];
        if let Some((name, _)) = thresholds
            .iter()
            .find(|(_, value)| !(value.is_finite() && *value > 0.0))
        {
            return Err(format!("{name} must be finite and positive"));
        }
        if !self.stand_up_pose.into_iter().all(f32::is_finite) {
            return Err("stand_up_pose must be finite".into());
        }
        let durations = [
            ("maximum_input_age", self.maximum_input_age),
            ("upright_duration", self.upright_duration),
            ("falling_duration", self.falling_duration),
            ("fallen_duration", self.fallen_duration),
            ("stand_up_stable_duration", self.stand_up_stable_duration),
        ];
        if let Some((name, _)) = durations.iter().find(|(_, duration)| duration.is_zero()) {
            return Err(format!("{name} must be positive"));
        }
        if self.stand_up_timeout <= self.upright_duration {
            return Err("stand_up_timeout must exceed upright_duration".into());
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
            let gap = observation.time.duration_since(last_time);
            if gap > MAXIMUM_FALL_DETECTION_AGE {
                // Consumers have already discarded the last posture.
                *self = Self::default();
            } else if gap > parameters.maximum_input_age {
                // Confirmation durations must be observed without interruption,
                // but the confirmed posture survives a short gap.
                self.candidate = None;
                self.stand_up_pose_since = None;
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
        };
        Some(FallDetection {
            time: observation.time,
            posture,
        })
    }

    fn update_readiness(
        &mut self,
        observation: &Observation,
        stand_up_requested: bool,
        parameters: &Parameters,
    ) {
        if posture::is_in_stand_up_pose(observation, parameters) {
            self.stand_up_pose_since.get_or_insert(observation.time);
        } else {
            self.stand_up_pose_since = None;
        }
        let stable = self.stand_up_pose_since.is_some_and(|since| {
            observation.time.duration_since(since) >= parameters.stand_up_stable_duration
        });
        // Preserve the accepted start pose while the requested policy moves away from it.
        self.ready_for_standup = self.state == Some(State::Fallen)
            && (stable || (self.ready_for_standup && stand_up_requested));
    }

    fn transition(
        &self,
        observation: &Observation,
        stand_up_requested: bool,
        parameters: &Parameters,
    ) -> Option<(State, Duration)> {
        let is_standing = posture::is_standing(observation, parameters);
        match self.state {
            Some(State::Upright) => (observation.tilt >= parameters.falling_tilt)
                .then_some((State::Falling, parameters.falling_duration)),
            Some(State::Fallen) if stand_up_requested && self.ready_for_standup => Some((
                State::StandingUp {
                    since: observation.time,
                },
                Duration::ZERO,
            )),
            Some(State::Fallen) => {
                is_standing.then_some((State::Upright, parameters.upright_duration))
            }
            // Lying or rolling is part of a recovery, so only standing or the timeout end it.
            Some(State::StandingUp { since }) => {
                if is_standing {
                    Some((State::Upright, parameters.upright_duration))
                } else if observation.time.duration_since(since) >= parameters.stand_up_timeout {
                    // Damp until the robot is confirmed standing or lying still.
                    Some((State::Falling, Duration::ZERO))
                } else {
                    None
                }
            }
            None | Some(State::Falling) => {
                if is_standing {
                    Some((State::Upright, parameters.upright_duration))
                } else if posture::is_lying_still(observation, parameters) {
                    Some((State::Fallen, parameters.fallen_duration))
                } else {
                    None
                }
            }
        }
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
        // Skipped samples show up as a gap in the detector.
        let Ok(observation) = Observation::from_low_state(&low_state, low_state.source_time) else {
            continue;
        };
        tilt_publisher.publish(&observation.tilt).await?;
        let parameters = parameters.snapshot();
        let parameters = parameters.typed();
        let now = node.clock().now();
        if observation.time > now
            || now.duration_since(observation.time) > parameters.maximum_input_age
        {
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
    use kinematics::joints::leg::LegJoints;

    use super::*;

    fn legs(hip_pitch: f32, knee: f32, ankle_up: f32) -> Joints<f32> {
        let leg = LegJoints {
            hip_pitch,
            hip_roll: 0.0,
            hip_yaw: 0.0,
            knee,
            ankle_up,
            ankle_down: 0.0,
        };
        Joints {
            left_leg: leg,
            right_leg: leg,
            ..Joints::fill(0.0)
        }
    }

    fn stand_up_pose() -> Joints<f32> {
        legs(-0.2, 0.4, -0.25)
    }

    fn twisted_pose() -> Joints<f32> {
        stand_up_pose() + Joints::fill(1.0)
    }

    /// Torso vertical, but hips and knees folded.
    fn seated_pose() -> Joints<f32> {
        legs(-1.6, 1.6, 0.0)
    }

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
            minimum_upright_foot_depth: 0.43,
            stand_up_timeout: Duration::from_secs(8),
            stand_up_pose: stand_up_pose(),
            maximum_stand_up_pose_deviation: 0.3,
            maximum_stand_up_joint_speed: 1.0,
            stand_up_stable_duration: Duration::from_millis(150),
        }
    }

    /// Feeds still samples every 10 ms in `[from, to]` and returns the last posture.
    fn run(
        detector: &mut Detector,
        from: i64,
        to: i64,
        tilt: f32,
        positions: Joints<f32>,
        stand_up_requested: bool,
    ) -> Option<Posture> {
        (from..=to)
            .step_by(10)
            .filter_map(|milliseconds| {
                let observation = Observation {
                    time: Time::from_nanos(milliseconds * 1_000_000),
                    tilt,
                    angular_speed: 0.0,
                    positions,
                    velocities: Joints::fill(0.0),
                };
                detector.update(observation, stand_up_requested, &parameters())
            })
            .last()
            .map(|fall_detection| fall_detection.posture)
    }

    fn fallen(ready_for_standup: bool) -> Option<Posture> {
        Some(Posture::Fallen { ready_for_standup })
    }

    #[test]
    fn parameters_are_valid() {
        assert_eq!(parameters().validate(), Ok(()));
    }

    #[test]
    fn fall_prepare_and_stand_up() {
        let mut detector = Detector::default();
        assert_eq!(run(&mut detector, 0, 50, 0.0, stand_up_pose(), false), None);
        assert_eq!(
            run(&mut detector, 60, 200, 0.0, stand_up_pose(), false),
            Some(Posture::Upright)
        );
        assert_eq!(
            run(&mut detector, 210, 240, 0.9, stand_up_pose(), false),
            Some(Posture::Falling)
        );
        assert_eq!(
            run(&mut detector, 250, 600, 1.5, twisted_pose(), false),
            fallen(false)
        );
        // Prepare reached the pose and settled.
        assert_eq!(
            run(&mut detector, 610, 800, 1.5, stand_up_pose(), false),
            fallen(true)
        );
        // The stand up policy moving the joints must not leave StandingUp.
        assert_eq!(
            run(&mut detector, 810, 2000, 0.9, twisted_pose(), true),
            Some(Posture::StandingUp)
        );
        assert_eq!(
            run(&mut detector, 2010, 2200, 0.1, stand_up_pose(), true),
            Some(Posture::Upright)
        );
    }

    #[test]
    fn stand_up_requires_ready() {
        let mut detector = Detector::default();
        run(&mut detector, 0, 300, 1.5, twisted_pose(), false);
        assert_eq!(
            run(&mut detector, 310, 500, 1.5, twisted_pose(), true),
            fallen(false)
        );
    }

    #[test]
    fn seated_robot_is_not_upright() {
        let mut detector = Detector::default();
        assert_eq!(run(&mut detector, 0, 500, 0.0, seated_pose(), false), None);
        run(&mut detector, 510, 700, 0.0, stand_up_pose(), false);
        run(&mut detector, 710, 800, 0.9, stand_up_pose(), false);
        assert_eq!(
            run(&mut detector, 810, 1500, 0.0, seated_pose(), false),
            Some(Posture::Falling)
        );
    }

    #[test]
    fn stand_up_timeout_while_lying_still_ends_fallen() {
        let mut detector = Detector::default();
        run(&mut detector, 0, 400, 1.5, stand_up_pose(), false);
        assert_eq!(
            run(&mut detector, 410, 420, 1.5, twisted_pose(), true),
            Some(Posture::StandingUp)
        );
        assert_eq!(
            run(&mut detector, 430, 8420, 1.5, twisted_pose(), true),
            Some(Posture::Falling)
        );
        // Still requested, but lying twisted: no retry without Prepare.
        assert_eq!(
            run(&mut detector, 8430, 8600, 1.5, twisted_pose(), true),
            fallen(false)
        );
    }

    #[test]
    fn stand_up_timeout_while_seated_damps() {
        let mut detector = Detector::default();
        run(&mut detector, 0, 400, 1.5, stand_up_pose(), false);
        run(&mut detector, 410, 420, 1.5, twisted_pose(), true);
        assert_eq!(
            run(&mut detector, 430, 10000, 0.0, seated_pose(), true),
            Some(Posture::Falling)
        );
    }

    #[test]
    fn short_sensor_gap_keeps_posture() {
        let mut detector = Detector::default();
        run(&mut detector, 0, 200, 0.0, stand_up_pose(), false);
        // Not a standing pose, so Upright could not be confirmed again.
        assert_eq!(
            run(&mut detector, 280, 280, 0.0, twisted_pose(), false),
            Some(Posture::Upright)
        );
    }

    #[test]
    fn long_sensor_gap_resets_posture() {
        let mut detector = Detector::default();
        run(&mut detector, 0, 200, 0.0, stand_up_pose(), false);
        assert_eq!(
            run(&mut detector, 400, 400, 0.0, stand_up_pose(), false),
            None
        );
    }
}
