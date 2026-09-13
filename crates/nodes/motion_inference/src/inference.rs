use crate::{
    config::{ARMS, Parameters, Policy},
    get_up::{GetUp, fast, slow},
    locomotion::{KickRequest, Locomotion, kick, walk},
    network::Network,
    observation::{SensorFrame, VelocityEstimator},
};
use ::kinematics::joints::Joints;
use anyhow::{Result, ensure};
use booster::{CommandType, MotorCommand};
use coordinate_systems::Ground;
use linear_algebra::Vector2;
use ros_z::{Message, time::Time};
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, path::Path, sync::Arc};

#[derive(Clone, Copy, Debug, Serialize, Deserialize, ros_z::Message)]
pub enum InferenceCommand {
    Stand,
    Walk {
        velocity: Vector2<Ground>,
        angular_velocity: f32,
    },
    Kick {
        soft: bool,
        request: KickRequest,
    },
    GetUp {
        fast: bool,
    },
}

#[derive(Clone, Serialize, Deserialize, Message)]
pub struct InferenceOutput {
    pub joints: Box<Joints<MotorCommand>>,
    pub mode: Mode,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, Message)]
pub enum Mode {
    Full,
    Body,
}

impl InferenceCommand {
    pub fn policy(self) -> Policy {
        match self {
            Self::Stand | Self::Walk { .. } => Policy::Walk,
            Self::Kick { soft: true, .. } => Policy::SoftKick,
            Self::Kick { soft: false, .. } => Policy::Kick,
            Self::GetUp { fast: false } => Policy::SlowGetUp,
            Self::GetUp { fast: true } => Policy::FastGetUp,
        }
    }

    fn is_moving(self, parameters: &Parameters) -> bool {
        match self {
            Self::Walk {
                velocity,
                angular_velocity,
            } => {
                velocity.x().abs() >= parameters.locomotion.minimum_forward_velocity
                    || velocity.y().abs() >= parameters.locomotion.minimum_lateral_velocity
                    || angular_velocity.abs()
                        >= parameters
                            .locomotion
                            .minimum_angular_velocity_degrees
                            .to_radians()
            }
            Self::Kick { .. } => true,
            _ => false,
        }
    }

    fn validate(self, parameters: &Parameters) -> Result<()> {
        match self {
            Self::Walk {
                velocity,
                angular_velocity,
            } => {
                ensure!(
                    velocity.inner.iter().all(|v| v.is_finite()) && angular_velocity.is_finite(),
                    "non-finite walking command"
                );
                ensure!(
                    (parameters.locomotion.forward_velocity_limits[0]
                        ..=parameters.locomotion.forward_velocity_limits[1])
                        .contains(&velocity.x())
                        && velocity.y().abs() <= parameters.locomotion.lateral_velocity_limit
                        && angular_velocity.abs() <= parameters.locomotion.angular_velocity_limit,
                    "walking command outside trained envelope"
                );
            }
            Self::Kick { request, .. } => ensure!(request.is_finite(), "invalid kick request"),
            _ => {}
        }
        Ok(())
    }
}

pub struct Inference {
    parameters: Arc<Parameters>,
    networks: HashMap<Policy, Network>,
    active: Option<Execution>,
    velocity: VelocityEstimator,
    previous_update: Option<Time>,
    last_motion: Option<Time>,
}

impl Inference {
    pub fn new(root: &Path, policies: &[Policy], parameters: Arc<Parameters>) -> Result<Self> {
        parameters.validate()?;
        ensure!(!policies.is_empty(), "no policies requested");
        let mut networks = HashMap::new();
        for &policy in policies {
            if let std::collections::hash_map::Entry::Vacant(entry) = networks.entry(policy) {
                entry.insert(Network::load(root, policy, &parameters)?);
            }
        }
        Ok(Self {
            parameters,
            networks,
            active: None,
            velocity: VelocityEstimator::default(),
            previous_update: None,
            last_motion: None,
        })
    }

    pub(crate) fn execute_request(
        &mut self,
        now: Time,
        sensor: &SensorFrame,
        request: InferenceCommand,
        velocity: VelocityEstimator,
    ) -> Result<InferenceOutput> {
        self.validate_update(now, sensor, request)?;
        self.velocity = velocity;
        self.activate(now, sensor, request.policy());
        let standing = self.advance_gait(now, request);
        self.infer(now, sensor, request, standing)
    }

    fn validate_update(
        &self,
        now: Time,
        sensor: &SensorFrame,
        request: InferenceCommand,
    ) -> Result<()> {
        sensor.validate(&self.parameters)?;
        request.validate(&self.parameters)?;
        let policy = request.policy();
        ensure!(
            self.networks.contains_key(&policy),
            "{policy:?} was not initialized"
        );
        if let Some(last) = self.previous_update {
            ensure!(now >= last, "controller time moved backwards");
        }
        Ok(())
    }

    fn activate(&mut self, now: Time, sensor: &SensorFrame, policy: Policy) {
        if let Some(active) = &mut self.active {
            if active.policy == policy {
                return;
            }
            if active.policy.is_locomotion() && policy.is_locomotion() {
                active.policy = policy;
                return;
            }
            if policy.is_locomotion() {
                self.last_motion = Some(now);
            }
        }
        self.active = Some(Execution::new(policy, sensor, now, self.parameters.clone()));
        self.previous_update = Some(now);
    }

    fn advance_gait(&mut self, now: Time, request: InferenceCommand) -> bool {
        if request.is_moving(&self.parameters) {
            self.last_motion = Some(now);
        }
        let standing = matches!(request, InferenceCommand::Stand)
            && self
                .last_motion
                .is_none_or(|last| now.duration_since(last) >= self.parameters.timing.stand_delay);
        let elapsed = self
            .previous_update
            .map_or(0.0, |last| now.duration_since(last).as_secs_f32());
        if let Some(active) = &mut self.active {
            active.advance(elapsed, standing);
        }
        self.previous_update = Some(now);
        standing
    }

    fn infer(
        &mut self,
        now: Time,
        sensor: &SensorFrame,
        request: InferenceCommand,
        standing: bool,
    ) -> Result<InferenceOutput> {
        let active = self
            .active
            .as_mut()
            .expect("policy activated before inference");
        let policy = active.policy;
        let observation = active.prepare_input(now, sensor, &self.velocity, request, standing);
        let raw_output = self
            .networks
            .get_mut(&policy)
            .expect("policy validated before activation")
            .run(&observation)?;
        let joints = active.decode(now, sensor, &raw_output);
        ensure!(joints_are_finite(joints), "non-finite decoded joints");
        Ok(InferenceOutput {
            joints: Box::new(joints),
            mode: if policy.is_locomotion() {
                Mode::Body
            } else {
                Mode::Full
            },
        })
    }
}

enum State {
    Locomotion(Locomotion),
    SlowGetUp(GetUp),
    FastGetUp(GetUp),
}

struct Execution {
    policy: Policy,
    state: State,
    started: Time,
    start_position: Joints,
}

impl Execution {
    fn new(policy: Policy, sensor: &SensorFrame, now: Time, parameters: Arc<Parameters>) -> Self {
        let state = match policy {
            Policy::Walk | Policy::Kick | Policy::SoftKick => {
                State::Locomotion(Locomotion::new(sensor, parameters))
            }
            Policy::SlowGetUp => State::SlowGetUp(GetUp::new(sensor, now, parameters)),
            Policy::FastGetUp => State::FastGetUp(GetUp::new(sensor, now, parameters)),
        };
        Self {
            policy,
            state,
            started: now,
            start_position: sensor.last_commanded_position,
        }
    }

    fn advance(&mut self, seconds: f32, standing: bool) {
        if let State::Locomotion(state) = &mut self.state {
            state.advance(seconds, standing);
        }
    }

    fn prepare_input(
        &mut self,
        now: Time,
        sensor: &SensorFrame,
        velocity: &VelocityEstimator,
        request: InferenceCommand,
        standing: bool,
    ) -> Vec<f32> {
        match &mut self.state {
            State::Locomotion(state) => match request {
                InferenceCommand::Kick { soft, request } => {
                    let ball_positions = state.record_kick_sample(sensor, request);
                    kick::Observation::new(
                        state,
                        sensor,
                        &velocity.walking,
                        standing,
                        request,
                        soft,
                        ball_positions,
                    )
                    .to_tensor()
                    .to_vec()
                }
                _ => {
                    state.record_walk_sample(sensor);
                    let (linear_velocity, angular_velocity) = match request {
                        InferenceCommand::Walk {
                            velocity,
                            angular_velocity,
                        } => (velocity, angular_velocity),
                        _ => (Vector2::zeros(), 0.0),
                    };
                    walk::Observation::new(
                        state,
                        &velocity.walking,
                        standing,
                        linear_velocity,
                        angular_velocity,
                    )
                    .to_tensor()
                    .to_vec()
                }
            },
            State::SlowGetUp(state) => slow::Observation::new(state, sensor, &velocity.get_up, now)
                .to_tensor()
                .to_vec(),
            State::FastGetUp(state) => fast::Observation::new(state, sensor, &velocity.get_up)
                .to_tensor()
                .to_vec(),
        }
    }

    fn decode(&mut self, now: Time, sensor: &SensorFrame, actions: &[f32]) -> Joints<MotorCommand> {
        match &mut self.state {
            State::Locomotion(state) => {
                let mut joints = state.decode(self.policy, actions, sensor);
                let ratio = (now.duration_since(self.started).as_secs_f32()
                    / state.parameters.timing.arm_blend_duration.as_secs_f32())
                .clamp(0.0, 1.0);
                for joint in ARMS {
                    joints[joint].position =
                        self.start_position[joint] * (1.0 - ratio) + joints[joint].position * ratio;
                }
                joints
            }
            State::SlowGetUp(state) | State::FastGetUp(state) => {
                state.decode(self.policy, actions, sensor)
            }
        }
    }
}

pub fn position_targets(
    position: Joints<f32>,
    kp: Joints<f32>,
    kd: Joints<f32>,
) -> Joints<MotorCommand> {
    position
        .enumerate()
        .map(|(joint, position)| MotorCommand {
            command_type: CommandType::Serial,
            position,
            kp: kp[joint],
            kd: kd[joint],
            weight: 1.0,
            ..MotorCommand::default()
        })
        .collect()
}

pub fn joints_are_finite(joints: Joints<MotorCommand>) -> bool {
    joints
        .into_iter()
        .flat_map(|joint| {
            [
                joint.position,
                joint.velocity,
                joint.torque,
                joint.kp,
                joint.kd,
                joint.weight,
            ]
        })
        .all(f32::is_finite)
}
