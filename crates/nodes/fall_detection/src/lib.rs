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

/// Angles in radians, speeds in radians per second, lengths in metres.
/// Confirmation durations must hold without interruption.
#[derive(Clone, Debug, Serialize, Deserialize, Message)]
#[serde(deny_unknown_fields)]
pub struct Parameters {
    /// Older sensor samples and motion commands are ignored.
    /// A longer gap between sensor samples restarts all confirmations.
    pub maximum_input_age: Duration,
    pub upright: UprightParameters,
    pub falling: FallingParameters,
    pub fallen: FallenParameters,
    pub stand_up: StandUpParameters,
}

/// Entering `Upright`: standing on extended legs. Once upright, only `falling` applies.
#[derive(Clone, Copy, Debug, Serialize, Deserialize, Message)]
#[serde(deny_unknown_fields)]
pub struct UprightParameters {
    pub maximum_torso_tilt: f32,
    pub maximum_torso_angular_speed: f32,
    pub maximum_leg_joint_speed: f32,
    /// Rejects sitting and kneeling with a vertical torso.
    pub minimum_torso_height_above_soles: f32,
    pub confirmation_duration: Duration,
}

/// Leaving `Upright`.
#[derive(Clone, Copy, Debug, Serialize, Deserialize, Message)]
#[serde(deny_unknown_fields)]
pub struct FallingParameters {
    pub minimum_torso_tilt: f32,
    pub confirmation_duration: Duration,
}

/// Entering `Fallen`: lying still.
#[derive(Clone, Copy, Debug, Serialize, Deserialize, Message)]
#[serde(deny_unknown_fields)]
pub struct FallenParameters {
    pub minimum_torso_tilt: f32,
    pub maximum_torso_angular_speed: f32,
    /// Applies to all joints except the head.
    pub maximum_joint_speed: f32,
    pub confirmation_duration: Duration,
}

/// Starting a stand up from `Fallen` and ending `StandingUp`.
#[derive(Clone, Copy, Debug, Serialize, Deserialize, Message)]
#[serde(deny_unknown_fields)]
pub struct StandUpParameters {
    /// Pose Prepare drives the joints into while fallen. The head is ignored.
    pub start_pose: Joints<f32>,
    /// Per joint, from `start_pose`.
    pub maximum_start_pose_deviation: f32,
    pub maximum_start_joint_speed: f32,
    /// How long the start pose must hold before `Fallen` reports `ready_for_standup`.
    pub confirmation_duration: Duration,
    /// Without confirmed standing, `StandingUp` ends as `Falling` after this.
    pub timeout: Duration,
}

impl Parameters {
    pub fn validate(&self) -> std::result::Result<(), String> {
        let Self {
            maximum_input_age,
            upright,
            falling,
            fallen,
            stand_up,
        } = self;
        if !(0.0 < upright.maximum_torso_tilt
            && upright.maximum_torso_tilt < falling.minimum_torso_tilt
            && falling.minimum_torso_tilt < fallen.minimum_torso_tilt
            && fallen.minimum_torso_tilt < PI)
        {
            return Err(
                "torso tilts must satisfy 0 < upright.maximum < falling.minimum \
                < fallen.minimum < pi"
                    .into(),
            );
        }
        let thresholds = [
            (
                "upright.maximum_torso_angular_speed",
                upright.maximum_torso_angular_speed,
            ),
            (
                "upright.maximum_leg_joint_speed",
                upright.maximum_leg_joint_speed,
            ),
            (
                "upright.minimum_torso_height_above_soles",
                upright.minimum_torso_height_above_soles,
            ),
            (
                "fallen.maximum_torso_angular_speed",
                fallen.maximum_torso_angular_speed,
            ),
            ("fallen.maximum_joint_speed", fallen.maximum_joint_speed),
            (
                "stand_up.maximum_start_pose_deviation",
                stand_up.maximum_start_pose_deviation,
            ),
            (
                "stand_up.maximum_start_joint_speed",
                stand_up.maximum_start_joint_speed,
            ),
        ];
        if let Some((name, _)) = thresholds
            .iter()
            .find(|(_, value)| !(value.is_finite() && *value > 0.0))
        {
            return Err(format!("{name} must be finite and positive"));
        }
        if !stand_up.start_pose.into_iter().all(f32::is_finite) {
            return Err("stand_up.start_pose must be finite".into());
        }
        let durations = [
            ("maximum_input_age", *maximum_input_age),
            (
                "upright.confirmation_duration",
                upright.confirmation_duration,
            ),
            (
                "falling.confirmation_duration",
                falling.confirmation_duration,
            ),
            ("fallen.confirmation_duration", fallen.confirmation_duration),
            (
                "stand_up.confirmation_duration",
                stand_up.confirmation_duration,
            ),
        ];
        if let Some((name, _)) = durations.iter().find(|(_, duration)| duration.is_zero()) {
            return Err(format!("{name} must be positive"));
        }
        if stand_up.timeout <= upright.confirmation_duration {
            return Err("stand_up.timeout must exceed upright.confirmation_duration".into());
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
    Fallen { ready_for_standup: bool },
    StandingUp { since: Time },
}

#[derive(Default)]
pub struct Detector {
    state: Option<State>,
    candidate: Option<(State, Time)>,
    start_pose_since: Option<Time>,
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
                self.start_pose_since = None;
            }
        }
        self.last_time = Some(observation.time);

        self.update_readiness(&observation, stand_up_requested, parameters);
        let transition = self.transition(&observation, stand_up_requested, parameters);
        self.debounce(transition, observation.time);
        let posture = match self.state? {
            State::Upright => Posture::Upright,
            State::Falling => Posture::Falling,
            State::Fallen { ready_for_standup } => Posture::Fallen { ready_for_standup },
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
        if posture::is_in_start_pose(observation, &parameters.stand_up) {
            self.start_pose_since.get_or_insert(observation.time);
        } else {
            self.start_pose_since = None;
        }
        let stable = self.start_pose_since.is_some_and(|since| {
            observation.time.duration_since(since) >= parameters.stand_up.confirmation_duration
        });
        if let Some(State::Fallen { ready_for_standup }) = &mut self.state {
            // Preserve the accepted start pose while the requested policy moves away from it.
            *ready_for_standup = stable || (*ready_for_standup && stand_up_requested);
        }
    }

    fn transition(
        &self,
        observation: &Observation,
        stand_up_requested: bool,
        parameters: &Parameters,
    ) -> Option<(State, Duration)> {
        let is_standing = posture::is_standing(observation, &parameters.upright);
        match self.state {
            Some(State::Upright) => (observation.tilt >= parameters.falling.minimum_torso_tilt)
                .then_some((State::Falling, parameters.falling.confirmation_duration)),
            Some(State::Fallen {
                ready_for_standup: true,
            }) if stand_up_requested => Some((
                State::StandingUp {
                    since: observation.time,
                },
                Duration::ZERO,
            )),
            Some(State::Fallen { .. }) => {
                is_standing.then_some((State::Upright, parameters.upright.confirmation_duration))
            }
            // Lying or rolling is part of a recovery, so only standing or the timeout end it.
            Some(State::StandingUp { since }) => {
                if is_standing {
                    Some((State::Upright, parameters.upright.confirmation_duration))
                } else if observation.time.duration_since(since) >= parameters.stand_up.timeout {
                    // Damp until the robot is confirmed standing or lying still.
                    Some((State::Falling, Duration::ZERO))
                } else {
                    None
                }
            }
            None | Some(State::Falling) => {
                if is_standing {
                    Some((State::Upright, parameters.upright.confirmation_duration))
                } else if posture::is_lying_still(observation, &parameters.fallen) {
                    Some((
                        State::Fallen {
                            ready_for_standup: false,
                        },
                        parameters.fallen.confirmation_duration,
                    ))
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

    fn start_pose() -> Joints<f32> {
        legs(-0.2, 0.4, -0.25)
    }

    fn twisted_pose() -> Joints<f32> {
        start_pose() + Joints::fill(1.0)
    }

    /// Torso vertical, but hips and knees folded.
    fn seated_pose() -> Joints<f32> {
        legs(-1.6, 1.6, 0.0)
    }

    fn parameters() -> Parameters {
        Parameters {
            maximum_input_age: Duration::from_millis(50),
            upright: UprightParameters {
                maximum_torso_tilt: 0.35,
                maximum_torso_angular_speed: 1.0,
                maximum_leg_joint_speed: 2.0,
                minimum_torso_height_above_soles: 0.43,
                confirmation_duration: Duration::from_millis(100),
            },
            falling: FallingParameters {
                minimum_torso_tilt: 0.8,
                confirmation_duration: Duration::from_millis(20),
            },
            fallen: FallenParameters {
                minimum_torso_tilt: 1.0,
                maximum_torso_angular_speed: 2.5,
                maximum_joint_speed: 1.0,
                confirmation_duration: Duration::from_millis(150),
            },
            stand_up: StandUpParameters {
                start_pose: start_pose(),
                maximum_start_pose_deviation: 0.3,
                maximum_start_joint_speed: 1.0,
                confirmation_duration: Duration::from_millis(150),
                timeout: Duration::from_secs(8),
            },
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
    fn base_parameters_are_valid() {
        let parameters: Parameters = json5::from_str(include_str!(
            "../../../../etc/parameters/base/fall_detection.json5"
        ))
        .unwrap();
        assert_eq!(parameters.validate(), Ok(()));
    }

    #[test]
    fn fall_prepare_and_stand_up() {
        let mut detector = Detector::default();
        assert_eq!(run(&mut detector, 0, 50, 0.0, start_pose(), false), None);
        assert_eq!(
            run(&mut detector, 60, 200, 0.0, start_pose(), false),
            Some(Posture::Upright)
        );
        assert_eq!(
            run(&mut detector, 210, 240, 0.9, start_pose(), false),
            Some(Posture::Falling)
        );
        assert_eq!(
            run(&mut detector, 250, 600, 1.5, twisted_pose(), false),
            fallen(false)
        );
        // Prepare reached the pose and settled.
        assert_eq!(
            run(&mut detector, 610, 800, 1.5, start_pose(), false),
            fallen(true)
        );
        // The stand up policy moving the joints must not leave StandingUp.
        assert_eq!(
            run(&mut detector, 810, 2000, 0.9, twisted_pose(), true),
            Some(Posture::StandingUp)
        );
        assert_eq!(
            run(&mut detector, 2010, 2200, 0.1, start_pose(), true),
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
        run(&mut detector, 510, 700, 0.0, start_pose(), false);
        run(&mut detector, 710, 800, 0.9, start_pose(), false);
        assert_eq!(
            run(&mut detector, 810, 1500, 0.0, seated_pose(), false),
            Some(Posture::Falling)
        );
    }

    #[test]
    fn stand_up_timeout_while_lying_still_ends_fallen() {
        let mut detector = Detector::default();
        run(&mut detector, 0, 400, 1.5, start_pose(), false);
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
        run(&mut detector, 0, 400, 1.5, start_pose(), false);
        run(&mut detector, 410, 420, 1.5, twisted_pose(), true);
        assert_eq!(
            run(&mut detector, 430, 10000, 0.0, seated_pose(), true),
            Some(Posture::Falling)
        );
    }

    #[test]
    fn short_sensor_gap_keeps_posture() {
        let mut detector = Detector::default();
        run(&mut detector, 0, 200, 0.0, start_pose(), false);
        // Not a standing pose, so Upright could not be confirmed again.
        assert_eq!(
            run(&mut detector, 280, 280, 0.0, twisted_pose(), false),
            Some(Posture::Upright)
        );
    }

    #[test]
    fn long_sensor_gap_resets_posture() {
        let mut detector = Detector::default();
        run(&mut detector, 0, 200, 0.0, start_pose(), false);
        assert_eq!(run(&mut detector, 400, 400, 0.0, start_pose(), false), None);
    }
}
