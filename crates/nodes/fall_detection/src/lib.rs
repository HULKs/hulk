use std::{f32::consts::PI, future::Future, pin::Pin, sync::Arc, time::Duration};

use booster::{JointsMotorState, LowState};
use color_eyre::{Result, eyre::ensure};
use kinematics::{
    forward::{left_sole_to_robot, right_sole_to_robot},
    joints::Joints,
};
use linear_algebra::Point3;
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
    pub standing_up: StandingUpParameters,
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

/// Entering `StandingUp` from `Fallen` and leaving it again.
#[derive(Clone, Copy, Debug, Serialize, Deserialize, Message)]
#[serde(deny_unknown_fields)]
pub struct StandingUpParameters {
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
            standing_up,
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
                "standing_up.maximum_start_pose_deviation",
                standing_up.maximum_start_pose_deviation,
            ),
            (
                "standing_up.maximum_start_joint_speed",
                standing_up.maximum_start_joint_speed,
            ),
        ];
        if let Some((name, _)) = thresholds
            .iter()
            .find(|(_, value)| !(value.is_finite() && *value > 0.0))
        {
            return Err(format!("{name} must be finite and positive"));
        }
        if !standing_up.start_pose.into_iter().all(f32::is_finite) {
            return Err("standing_up.start_pose must be finite".into());
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
                "standing_up.confirmation_duration",
                standing_up.confirmation_duration,
            ),
        ];
        if let Some((name, _)) = durations.iter().find(|(_, duration)| duration.is_zero()) {
            return Err(format!("{name} must be positive"));
        }
        if standing_up.timeout <= upright.confirmation_duration {
            return Err("standing_up.timeout must exceed upright.confirmation_duration".into());
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

    /// Standing on extended legs. A level torso alone also matches sitting or kneeling.
    fn is_standing(&self, parameters: &UprightParameters) -> bool {
        // Robot-relative kinematics, needing neither ground nor odometry estimates.
        let left_height = -(left_sole_to_robot(&self.positions.left_leg) * Point3::origin()).z();
        let right_height = -(right_sole_to_robot(&self.positions.right_leg) * Point3::origin()).z();
        self.tilt < parameters.maximum_torso_tilt
            && self.angular_speed < parameters.maximum_torso_angular_speed
            && self
                .velocities
                .left_leg
                .into_iter()
                .chain(self.velocities.right_leg)
                .all(|velocity| velocity.abs() < parameters.maximum_leg_joint_speed)
            && left_height > parameters.minimum_torso_height_above_soles
            && right_height > parameters.minimum_torso_height_above_soles
    }

    /// The head does not affect lying still.
    fn is_lying_still(&self, parameters: &FallenParameters) -> bool {
        self.tilt >= parameters.minimum_torso_tilt
            && self.angular_speed < parameters.maximum_torso_angular_speed
            && self
                .velocities
                .body()
                .into_iter()
                .all(|velocity| velocity.abs() < parameters.maximum_joint_speed)
    }

    /// Compares with the configured Prepare reference, ignoring the head. This is a pose
    /// envelope, not proof of collision clearance or membership in the policy's training set.
    fn is_in_start_pose(&self, parameters: &StandingUpParameters) -> bool {
        (self.positions - parameters.start_pose)
            .body()
            .into_iter()
            .all(|deviation| deviation.abs() < parameters.maximum_start_pose_deviation)
            && self
                .velocities
                .body()
                .into_iter()
                .all(|velocity| velocity.abs() < parameters.maximum_start_joint_speed)
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
        if observation.is_in_start_pose(&parameters.standing_up) {
            self.start_pose_since.get_or_insert(observation.time);
        } else {
            self.start_pose_since = None;
        }
        let stable = self.start_pose_since.is_some_and(|since| {
            observation.time.duration_since(since) >= parameters.standing_up.confirmation_duration
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
        let is_standing = observation.is_standing(&parameters.upright);
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
                } else if observation.time.duration_since(since) >= parameters.standing_up.timeout {
                    // Damp until the robot is confirmed standing or lying still.
                    Some((State::Falling, Duration::ZERO))
                } else {
                    None
                }
            }
            None | Some(State::Falling) => {
                if is_standing {
                    Some((State::Upright, parameters.upright.confirmation_duration))
                } else if observation.is_lying_still(&parameters.fallen) {
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
mod tests;
