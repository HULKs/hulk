mod parameters;

pub use parameters::Parameters;

use std::{future::Future, pin::Pin, sync::Arc, time::Duration};

use booster::{JointsMotorState, LowState};
use color_eyre::Result;
use kinematics::{
    forward::{left_sole_to_robot, right_sole_to_robot},
    joints::body::{BodyJoints, LowerBodyJoints},
};
use linear_algebra::Point3;
use ros_z::{
    prelude::*,
    qos::{QosHistory, QosReliability},
    time::Time,
};
use types::{
    fall_detection::{FallDetection, Posture},
    motion_command::MotionCommand,
};

use crate::parameters::{FallenParameters, StandingUpParameters, UprightParameters};

#[derive(Clone, Copy, Debug)]
pub struct Observation {
    pub time: Time,
    pub tilt: f32,
    pub pitch: f32,
    pub angular_velocity: f32,
    pub positions: BodyJoints<f32>,
    pub velocities: BodyJoints<f32>,
}

impl Observation {
    pub fn from_low_state(low_state: &LowState, time: Time) -> Result<Self> {
        let motors = low_state.serial_motor_states()?;
        let positions = motors.positions().body();
        let velocities = motors.velocities().body();
        let roll_pitch_yaw = low_state.imu_state.roll_pitch_yaw;
        let gyro = low_state.imu_state.angular_velocity;

        // Angle between the torso's z axis and the vertical.
        let tilt = (roll_pitch_yaw.x().cos() * roll_pitch_yaw.y().cos())
            .clamp(-1.0, 1.0)
            .acos();
        let angular_velocity = gyro.inner.norm();

        Ok(Self {
            time,
            tilt,
            pitch: roll_pitch_yaw.y(),
            angular_velocity,
            positions,
            velocities,
        })
    }

    fn is_standing(&self, parameters: &UprightParameters) -> bool {
        let torso_height_above_left_sole =
            -(left_sole_to_robot(&self.positions.left_leg) * Point3::origin()).z();
        let torso_height_above_right_sole =
            -(right_sole_to_robot(&self.positions.right_leg) * Point3::origin()).z();

        self.tilt < parameters.maximum_torso_tilt
            && self.angular_velocity < parameters.maximum_torso_angular_velocity
            && LowerBodyJoints::from(self.velocities)
                .into_iter()
                .all(|velocity| velocity.abs() < parameters.maximum_leg_joint_velocity)
            && torso_height_above_left_sole > parameters.minimum_torso_height_above_soles
            && torso_height_above_right_sole > parameters.minimum_torso_height_above_soles
    }

    fn is_lying_still(&self, parameters: &FallenParameters) -> bool {
        self.tilt >= parameters.minimum_torso_tilt
            && self.angular_velocity < parameters.maximum_torso_angular_velocity
            && self
                .velocities
                .into_iter()
                .all(|velocity| velocity.abs() < parameters.maximum_joint_velocity)
            && self.velocities.into_iter().map(f32::abs).sum::<f32>()
                < parameters.maximum_joint_velocity_sum
    }

    fn is_in_start_pose(&self, parameters: &StandingUpParameters) -> bool {
        // abs(sin(pitch)) is the vertical projection of the torso's forward axis.
        // Near one means lying on the front or back, regardless of yaw.
        self.pitch.sin().abs() > parameters.maximum_start_pose_torso_deviation.cos()
            && self.angular_velocity < parameters.maximum_start_pose_torso_angular_velocity
            && (self.positions - parameters.start_pose)
                .into_iter()
                .all(|deviation| {
                    deviation.abs() < parameters.maximum_start_pose_joint_position_deviation
                })
            && self
                .velocities
                .into_iter()
                .all(|velocity| velocity.abs() < parameters.maximum_start_pose_joint_velocity)
            && self.velocities.into_iter().map(f32::abs).sum::<f32>()
                < parameters.maximum_start_pose_joint_velocity_sum
    }
}

#[derive(Default)]
pub struct Detector {
    /// The confirmed posture and when it was entered.
    posture: Option<(Posture, Time)>,
    candidate: Option<(Posture, Time)>,
    start_pose_since: Option<Time>,
    stand_up_movement_since: Option<Time>,
    /// Enables a return to `Fallen` for the rest of this standup attempt.
    stand_up_movement_confirmed: bool,
    last_time: Option<Time>,
}

impl Detector {
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
            let time_since_last_observation = observation.time.duration_since(last_time);
            if time_since_last_observation > parameters.detection_lifetime {
                // Consumers have already discarded the last posture.
                *self = Self::default();
            } else if time_since_last_observation > parameters.maximum_observation_age {
                // Confirmation durations must be observed without interruption,
                // but the confirmed posture survives a short gap.
                self.candidate = None;
                self.start_pose_since = None;
                self.stand_up_movement_since = None;
            }
        }
        self.last_time = Some(observation.time);

        self.update_readiness(&observation, stand_up_requested, parameters);
        self.update_stand_up_movement(&observation, parameters);
        let transition = self.transition(&observation, stand_up_requested, parameters);
        self.debounce(transition, observation.time);
        let (posture, _) = self.posture?;
        Some(FallDetection {
            time: observation.time,
            valid_until: observation.time + parameters.detection_lifetime,
            posture,
        })
    }

    fn update_readiness(
        &mut self,
        observation: &Observation,
        stand_up_requested: bool,
        parameters: &Parameters,
    ) {
        if matches!(self.posture, Some((Posture::Fallen { .. }, _)))
            && observation.is_in_start_pose(&parameters.standing_up)
        {
            self.start_pose_since.get_or_insert(observation.time);
        } else {
            self.start_pose_since = None;
        }
        let stable = self.start_pose_since.is_some_and(|since| {
            observation.time.duration_since(since) >= parameters.standing_up.confirmation_duration
        });
        if let Some((
            Posture::Fallen {
                is_ready_for_standup: ready_for_standup,
            },
            _,
        )) = &mut self.posture
        {
            // The policy may leave the start pose before the request arrives here.
            *ready_for_standup = stable || (*ready_for_standup && stand_up_requested);
        }
    }

    fn update_stand_up_movement(&mut self, observation: &Observation, parameters: &Parameters) {
        if matches!(self.posture, Some((Posture::StandingUp, _)))
            && !self.stand_up_movement_confirmed
            && !observation.is_lying_still(&parameters.fallen)
        {
            let since = *self.stand_up_movement_since.get_or_insert(observation.time);
            self.stand_up_movement_confirmed = observation.time.duration_since(since)
                >= parameters.standing_up.movement_confirmation_duration;
        } else {
            self.stand_up_movement_since = None;
        }
    }

    fn transition(
        &self,
        observation: &Observation,
        stand_up_requested: bool,
        parameters: &Parameters,
    ) -> Option<(Posture, Duration)> {
        let is_standing = observation.is_standing(&parameters.upright);
        let is_tilted = observation.tilt >= parameters.falling.minimum_torso_tilt;
        let fallen_transition = observation.is_lying_still(&parameters.fallen).then_some((
            Posture::Fallen {
                is_ready_for_standup: false,
            },
            parameters.fallen.confirmation_duration,
        ));
        match self.posture {
            Some((Posture::Upright, _)) => {
                is_tilted.then_some((Posture::Falling, parameters.falling.confirmation_duration))
            }
            Some((
                Posture::Fallen {
                    is_ready_for_standup: true,
                },
                _,
            )) if stand_up_requested => Some((Posture::StandingUp, Duration::ZERO)),
            Some((Posture::Fallen { .. }, _)) => {
                is_standing.then_some((Posture::Upright, parameters.upright.confirmation_duration))
            }
            Some((Posture::StandingUp, since)) => {
                if observation.time.duration_since(since) >= parameters.standing_up.timeout {
                    Some((Posture::Falling, Duration::ZERO))
                } else if is_standing {
                    Some((Posture::Upright, parameters.upright.confirmation_duration))
                } else if self.stand_up_movement_confirmed {
                    fallen_transition
                } else {
                    None
                }
            }
            None | Some((Posture::Falling, _)) => {
                if is_standing {
                    Some((Posture::Upright, parameters.upright.confirmation_duration))
                } else {
                    fallen_transition
                }
            }
        }
    }

    fn debounce(&mut self, transition: Option<(Posture, Duration)>, time: Time) {
        let Some((next, duration)) = transition else {
            self.candidate = None;
            return;
        };
        let since = match self.candidate {
            Some((candidate, since)) if candidate == next => since,
            _ => time,
        };
        if time.duration_since(since) >= duration {
            self.posture = Some((next, time));
            self.candidate = None;
            self.stand_up_movement_since = None;
            self.stand_up_movement_confirmed = false;
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
    let maximum_joint_speed_publisher = node
        .publisher::<f32>("debug/fall_detection/maximum_joint_speed")
        .build()
        .await?;
    let joint_speed_sum_publisher = node
        .publisher::<f32>("debug/fall_detection/joint_speed_sum")
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
        if observation.time > now
            || now.duration_since(observation.time) > parameters.maximum_observation_age
        {
            continue;
        }

        let stand_up_requested =
            motion_command_cache
                .get_latest_with_stamp()
                .is_some_and(|(time, command)| {
                    matches!(*command, MotionCommand::StandUp)
                        && time <= now
                        && now.duration_since(time) <= parameters.maximum_observation_age
                });
        if let Some(fall_detection) = detector.update(observation, stand_up_requested, parameters) {
            fall_detection_publisher.publish(&fall_detection).await?;
        }

        // Match the magnitude comparisons in the fallen and readiness guards.
        // Observation already excludes the head.
        let maximum_joint_speed = observation
            .velocities
            .into_iter()
            .map(f32::abs)
            .fold(0.0, f32::max);
        let joint_speed_sum = observation.velocities.into_iter().map(f32::abs).sum();
        tilt_publisher
            .publish_with_source_time(&observation.tilt, observation.time)
            .await?;
        maximum_joint_speed_publisher
            .publish_with_source_time(&maximum_joint_speed, observation.time)
            .await?;
        joint_speed_sum_publisher
            .publish_with_source_time(&joint_speed_sum, observation.time)
            .await?;
    }
}

#[cfg(test)]
mod tests;
