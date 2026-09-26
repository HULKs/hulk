use std::{
    future::Future,
    pin::Pin,
    sync::Arc,
    time::{Duration, Instant},
};

use booster::{JointsMotorState, LowState};
use color_eyre::{Report, Result};
use kinematics::joints::{Joints, body::LowerBodyJoints};
use nalgebra::UnitQuaternion;
use ros_z::{
    Message,
    prelude::*,
    qos::{QosDurability, QosHistory, QosReliability},
    time::Time,
};
use serde::{Deserialize, Serialize};
use types::joint_limits::JointLimits;
use types::motor_command::MotorCommand;

pub use crate::config::Parameters;
use crate::{
    config::Policy,
    inference::{
        GetUpCommand, Inference, InferenceCommand, InferenceOutput, InferenceRequest,
        InferenceResponse, KickCommand, WalkCommand,
    },
    observation::{SensorFrame, VelocityEstimator},
};

pub const GETUP_INFERENCE_SERVICE: &str = "motion_inference/infer_getup";
pub const KICK_INFERENCE_SERVICE: &str = "motion_inference/infer_kick";
pub const WALK_INFERENCE_SERVICE: &str = "motion_inference/infer_walk";

macro_rules! inference_service {
    ($name:ident, $command:ty, $joints:ty) => {
        pub struct $name;
        impl Service for $name {
            type Request = InferenceRequest<$command>;
            type Response = InferenceResult<InferenceResponse<$joints>>;
        }
        impl ServiceTypeInfo for $name {
            fn service_type_info() -> TypeInfo {
                let descriptor = ros_z_schema::ServiceDef::new(
                    concat!(module_path!(), "::", stringify!($name)),
                    <Self as Service>::Request::type_name(),
                    <Self as Service>::Response::type_name(),
                )
                .expect("static inference service descriptor");
                TypeInfo::new(
                    descriptor.type_name.as_str(),
                    ros_z_schema::compute_hash(&descriptor).expect("static service hash"),
                )
            }
        }
    };
}

inference_service!(
    WalkInferenceService,
    WalkCommand,
    LowerBodyJoints<MotorCommand>
);
inference_service!(
    KickInferenceService,
    KickCommand,
    LowerBodyJoints<MotorCommand>
);
inference_service!(GetUpInferenceService, GetUpCommand, Joints<MotorCommand>);

use crate::services::{InferenceReply, Requests};
pub const STATUS_TOPIC: &str = "motion_inference/status";
pub const TIMING_TOPIC: &str = "motion_inference/timing";

pub type InferenceResult<T> = std::result::Result<T, InferenceError>;

#[derive(Clone, Debug, Serialize, Deserialize, Message, thiserror::Error)]
pub enum InferenceError {
    #[error("inference request or selected sensor frame expired")]
    Expired,
    #[error("inference request was superseded")]
    Superseded,
    #[error("inference is waiting for fresh sensors and validated joint limits")]
    Unavailable,
    #[error("motion inference fault: {source:#}")]
    Fault {
        #[serde(with = "ros_z::message::report")]
        source: Arc<Report>,
    },
}

#[derive(Clone, Serialize, Deserialize, Message)]
pub struct Status {
    pub time: Time,
    pub state: State,
}
#[derive(Clone, Serialize, Deserialize, Message)]
pub enum State {
    Idle,
    Initialized,
    Fault { reason: String },
}

#[derive(Clone, Serialize, Deserialize, Message)]
pub struct Timing {
    pub generation: u64,
    pub policy: Policy,
    pub requested_at: Time,
    pub received_at: Time,
    pub started_at: Time,
    pub completed_at: Time,
    pub sensor_time: Time,
    pub compute_duration: Duration,
    pub accepted: bool,
    pub error: Option<String>,
}

struct Pending {
    request: InferenceRequest<InferenceCommand>,
    reply: InferenceReply,
    received_at: Time,
}
impl Pending {
    async fn reject(self, error: InferenceError) {
        self.reply.respond(Err(error)).await;
    }
}

pub fn run_boxed(ctx: Arc<Context>) -> Pin<Box<dyn Future<Output = Result<()>> + Send>> {
    Box::pin(run(ctx))
}

async fn run(ctx: Arc<Context>) -> Result<()> {
    let node = ctx.create_node("motion_inference").build().await?;
    let parameters = node.bind_parameter_as::<Parameters>("motion_inference")?;
    let startup = parameters.snapshot().typed.clone();
    parameters.add_validation_hook(move |candidate| {
        candidate.validate().map_err(|error| format!("{error:#}"))?;
        if candidate != startup.as_ref() {
            return Err("motion inference parameter changes require a restart".into());
        }
        Ok(())
    })?;
    let qos = QosProfile {
        history: QosHistory::from_depth(1),
        reliability: QosReliability::BestEffort,
        ..Default::default()
    };
    let statuses = node
        .publisher::<Status>(STATUS_TOPIC)
        .qos(QosProfile {
            durability: QosDurability::TransientLocal,
            ..qos
        })
        .build()
        .await?;
    let timings = node
        .publisher::<Timing>(TIMING_TOPIC)
        .qos(qos)
        .build()
        .await?;
    statuses
        .publish(&Status {
            time: node.clock().now(),
            state: State::Idle,
        })
        .await?;
    let inference_parameters = parameters.snapshot().typed.clone();
    let initialized = tokio::task::spawn_blocking(move || {
        Inference::new(
            &inference_parameters.neural_networks_folder,
            &Policy::ALL,
            inference_parameters.clone(),
        )
    })
    .await?;
    let controller = match initialized {
        Ok(controller) => Some(controller),
        Err(error) => {
            statuses
                .publish(&Status {
                    time: node.clock().now(),
                    state: State::Fault {
                        reason: format!("{error:#}"),
                    },
                })
                .await?;
            return Err(error);
        }
    };
    let sensors = node
        .subscriber::<LowState>("inputs/low_state")
        .qos(qos)
        .build()
        .await?;
    let limits = node
        .subscriber::<JointLimits>("joint_limits")
        .qos(QosProfile {
            durability: QosDurability::TransientLocal,
            ..qos
        })
        .build()
        .await?;
    let mut requests = Requests::new(&node, qos).await?;
    statuses
        .publish(&Status {
            time: node.clock().now(),
            state: State::Initialized,
        })
        .await?;
    Runtime::new(
        parameters.snapshot().typed.clone(),
        node.clock().clone(),
        controller,
    )
    .run(sensors, limits, &mut requests, &statuses, &timings)
    .await
}

type WorkerResult = (Inference, Result<InferenceOutput>, Duration);

struct Runtime {
    parameters: Arc<Parameters>,
    clock: ros_z::time::Clock,
    controller: Option<Inference>,
    worker: Option<tokio::task::JoinHandle<WorkerResult>>,
    active: Option<(Pending, Time, Time)>,
    pending: Option<Pending>,
    sensor: Option<SensorFrame>,
    joint_limits: Option<Arc<JointLimits>>,
    velocity: VelocityEstimator,
    last_position: Option<(Policy, Joints<f32>)>,
    generation: u64,
    fault: Option<Arc<Report>>,
}

impl Runtime {
    fn new(
        parameters: Arc<Parameters>,
        clock: ros_z::time::Clock,
        controller: Option<Inference>,
    ) -> Self {
        Self {
            parameters,
            clock,
            controller,
            worker: None,
            active: None,
            pending: None,
            sensor: None,
            joint_limits: None,
            velocity: VelocityEstimator::default(),
            last_position: None,
            generation: 0,
            fault: None,
        }
    }

    async fn run(
        mut self,
        sensors: Subscriber<LowState>,
        limits: Subscriber<JointLimits>,
        requests: &mut Requests,
        statuses: &Publisher<Status>,
        timings: &Publisher<Timing>,
    ) -> Result<()> {
        let mut published_fault: Option<String> = None;
        loop {
            tokio::select! {
                received = sensors.recv_with_metadata() => {
                    let received = received?;
                    self.receive_sensor(&received, received.source_time);
                }
                received = limits.recv() => {
                    self.receive_limits(received?);
                }
                received = requests.receive() => {
                    let (request, reply) = received?;
                    self.enqueue(Pending {
                        request,
                        reply,
                        received_at: self.clock.now(),
                    })
                    .await;
                }
                completed = async { self.worker.as_mut().expect("active worker").await }, if self.worker.is_some() => {
                    self.complete(completed?, timings).await?;
                }
                _ = std::future::ready(()), if self.worker.is_none() && self.pending.is_some() => {
                    self.dispatch().await;
                }
            }
            let fault = self.fault.as_ref().map(|error| format!("{error:#}"));

            if fault != published_fault {
                let state = match &fault {
                    Some(reason) => State::Fault {
                        reason: reason.clone(),
                    },
                    None => State::Initialized,
                };

                statuses
                    .publish(&Status {
                        time: self.clock.now(),
                        state,
                    })
                    .await?;

                published_fault = fault;
            }
        }
    }

    fn receive_sensor(&mut self, low: &LowState, time: Time) {
        let sensor = match sensor_frame(low, time) {
            Ok(sensor) => sensor,
            Err(error) => {
                self.sensor = None;
                self.fault = Some(Arc::new(error));
                return;
            }
        };
        if let Err(error) = sensor.validate(&self.parameters) {
            self.sensor = None;
            self.fault = Some(Arc::new(error));
            return;
        }
        let now = self.clock.now();
        if sensor.timestamp > now {
            self.sensor = None;
            self.fault = Some(Arc::new(Report::msg("sensor timestamp is in the future")));
            return;
        }
        if now.duration_since(sensor.timestamp) > self.parameters.timing.maximum_sensor_age
            || self
                .sensor
                .as_ref()
                .is_some_and(|old| sensor.timestamp <= old.timestamp)
        {
            return;
        }
        if let Err(error) = self.velocity.update(&sensor, &self.parameters) {
            self.fault = Some(Arc::new(error));
        }
        self.sensor = Some(sensor);
    }

    fn receive_limits(&mut self, limits: JointLimits) {
        match limits.validate() {
            Ok(()) => self.joint_limits = Some(Arc::new(limits)),
            Err(reason) => {
                self.joint_limits = None;
                self.fault = Some(Arc::new(Report::msg(reason)));
            }
        }
    }

    async fn enqueue(&mut self, pending: Pending) {
        let request = pending.request;
        if request.generation < self.generation {
            pending.reject(InferenceError::Superseded).await;
            return;
        }
        if request.requested_at > self.clock.now() || self.clock.now() >= request.valid_until {
            pending.reject(InferenceError::Expired).await;
            return;
        }
        if request.generation > self.generation {
            self.generation = request.generation;
            if let Some(controller) = &mut self.controller {
                controller.reset();
            }
            self.last_position = None;
            self.fault = None;
        }
        if let Some(old) = self.pending.replace(pending) {
            old.reject(InferenceError::Superseded).await;
        }
    }

    async fn dispatch(&mut self) {
        let pending = self.pending.take().expect("pending request");
        let now = self.clock.now();
        if now >= pending.request.valid_until {
            pending.reject(InferenceError::Expired).await;
            return;
        }
        if let Some(source) = &self.fault {
            pending
                .reject(InferenceError::Fault {
                    source: source.clone(),
                })
                .await;
            return;
        }
        let (Some(sensor), Some(limits)) = (&self.sensor, &self.joint_limits) else {
            pending.reject(InferenceError::Unavailable).await;
            return;
        };
        if sensor.validate_at(now, &self.parameters).is_err() {
            pending.reject(InferenceError::Expired).await;
            return;
        }
        let mut sensor = sensor.clone();
        sensor.last_commanded_position = match self.last_position {
            Some((policy, _))
                if policy.is_locomotion()
                    && matches!(pending.request.command, InferenceCommand::GetUp(_)) =>
            {
                sensor.position
            }
            Some((_, position)) => position,
            None => sensor.position,
        };
        let limits = limits.clone();
        let velocity = self.velocity.clone();
        let request = pending.request;
        let mut controller = self.controller.take().expect("idle controller");
        let clock = self.clock.clone();
        let parameters = self.parameters.clone();
        self.active = Some((pending, now, sensor.timestamp));
        self.worker = Some(tokio::task::spawn_blocking(move || {
            let start = Instant::now();
            let result = controller.execute_request(
                clock.now(),
                &sensor,
                request.command,
                velocity,
                &limits,
                parameters,
            );
            (controller, result, start.elapsed())
        }));
    }

    async fn complete(
        &mut self,
        completed: WorkerResult,
        timings: &Publisher<Timing>,
    ) -> Result<()> {
        self.worker = None;
        let (pending, started_at, sensor_time) = self.active.take().expect("active request");
        let (mut controller, result, compute_duration) = completed;
        let now = self.clock.now();
        let valid = pending.request.generation == self.generation
            && now < pending.request.valid_until
            && now >= sensor_time
            && now.duration_since(sensor_time) <= self.parameters.timing.maximum_sensor_age;
        let result = if let Some(source) = &self.fault {
            controller.reset();
            self.last_position = None;
            Err(InferenceError::Fault {
                source: source.clone(),
            })
        } else if !valid {
            controller.reset();
            self.last_position = None;
            Err(InferenceError::Expired)
        } else {
            result.map_err(|error| {
                let source = Arc::new(error);
                self.fault = Some(source.clone());
                InferenceError::Fault { source }
            })
        };
        if let Ok(output) = &result {
            self.last_position = Some((
                output.execution.policy,
                output
                    .joints
                    .as_ref()
                    .into_iter()
                    .map(|joint| joint.position)
                    .collect(),
            ));
        }
        self.controller = Some(controller);
        timings
            .publish(&Timing {
                generation: pending.request.generation,
                policy: pending.request.command.policy(),
                requested_at: pending.request.requested_at,
                received_at: pending.received_at,
                started_at,
                completed_at: now,
                sensor_time,
                compute_duration,
                accepted: result.is_ok(),
                error: result.as_ref().err().map(ToString::to_string),
            })
            .await?;
        pending.reply.respond(result).await;
        Ok(())
    }
}

fn sensor_frame(low_state: &LowState, timestamp: Time) -> Result<SensorFrame> {
    let motors = low_state.serial_motor_states()?;
    let angles = low_state.imu_state.roll_pitch_yaw;
    let orientation = UnitQuaternion::from_euler_angles(angles.x(), angles.y(), angles.z());
    Ok(SensorFrame {
        timestamp,
        position: motors.positions(),
        velocity: motors.velocities(),
        orientation: orientation.into_inner(),
        gyro: low_state.imu_state.angular_velocity,
        last_commanded_position: motors.positions(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sensor_frame_seeds_previous_targets_from_measured_positions() {
        let positions: Joints<f32> = (0..crate::config::JOINT_COUNT)
            .map(|index| index as f32 * 0.01)
            .collect();
        let low_state = LowState {
            motor_state_serial: positions
                .into_iter()
                .map(|position| booster::MotorState {
                    position,
                    ..Default::default()
                })
                .collect(),
            ..Default::default()
        };

        let sensor = sensor_frame(&low_state, Time::zero()).unwrap();

        assert_eq!(sensor.position, positions);
        assert_eq!(sensor.last_commanded_position, positions);
    }
}
