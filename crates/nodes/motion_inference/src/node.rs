use std::{future::Future, pin::Pin, sync::Arc, time::Duration};

use color_eyre::{Report, Result};
use nalgebra::UnitQuaternion;
use serde::{Deserialize, Serialize};
use tracing::warn;

use booster::{JointsMotorState, LowState};
use kinematics::joints::{Joints, body::LowerBodyJoints};
use ros_z::{
    Message,
    prelude::*,
    pubsub::Received,
    qos::{QosDurability, QosHistory, QosReliability},
    service::RequestId,
    time::Time,
};
use types::{
    joint_limits::JointLimits,
    motor_command::MotorCommand,
    walking_velocity_limits::{WALKING_VELOCITY_LIMITS_TOPIC, WalkingVelocityLimits},
};

pub use crate::config::Parameters;
use crate::{
    config::Policy,
    inference::{GetUpCommand, Inference, InferenceCommand, KickCommand, WalkCommand},
    observation::{SensorFrame, VelocityEstimator},
    trace::{Event, InputSources, PublicationOrigin, Record, TRACE_TOPIC, Trace},
};

pub const GETUP_INFERENCE_SERVICE: &str = "motion_inference/infer_getup";
pub const KICK_INFERENCE_SERVICE: &str = "motion_inference/infer_kick";
pub const WALK_INFERENCE_SERVICE: &str = "motion_inference/infer_walk";

#[derive(Clone, Copy, Debug, Serialize, Deserialize, Message)]
pub struct InferenceRequest<T> {
    pub command: T,
    pub reset: bool,
    /// Node-time deadline; caller and server must share the same clock domain.
    pub deadline: Time,
}

impl<T> InferenceRequest<T> {
    pub fn new(command: T, reset: bool, now: Time, timeout: Duration) -> Self {
        Self {
            command,
            reset,
            deadline: now + timeout,
        }
    }

    pub(crate) fn map<U>(self, map: impl FnOnce(T) -> U) -> InferenceRequest<U> {
        InferenceRequest {
            command: map(self.command),
            reset: self.reset,
            deadline: self.deadline,
        }
    }
}

macro_rules! inference_service {
    ($name:ident, $command:ty, $joints:ty) => {
        pub struct $name;
        impl Service for $name {
            type Request = InferenceRequest<$command>;
            type Response = InferenceResult<$joints>;
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

use crate::services::{Pending, Requests};
pub const STATUS_TOPIC: &str = "motion_inference/status";
pub const TIMING_TOPIC: &str = "motion_inference/timing";

pub type InferenceResult<T> = std::result::Result<T, InferenceError>;

#[derive(Clone, Debug, Serialize, Deserialize, Message, thiserror::Error)]
pub enum InferenceError {
    #[error("inference is waiting for sensors and validated joint limits")]
    Unavailable,
    #[error("inference request deadline expired")]
    DeadlineExpired,
    #[error("inference input {input}: {reason}")]
    InputUnavailable { input: String, reason: String },
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
    pub request_id: RequestId,
    pub policy: Policy,
    pub received_at: Time,
    pub started_at: Time,
    pub completed_at: Time,
    pub sensor_time: Time,
    /// Elapsed node time, which may be zero while simulation is paused.
    pub compute_duration: Duration,
    pub accepted: bool,
    pub error: Option<String>,
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
    let traces = node
        .publisher::<Record>(TRACE_TOPIC)
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
    let walking_velocity_limits = node
        .subscriber::<WalkingVelocityLimits>(WALKING_VELOCITY_LIMITS_TOPIC)
        .qos(QosProfile {
            durability: QosDurability::TransientLocal,
            ..qos
        })
        .build()
        .await?;
    let (trace, records) = Trace::channel(node.clock().clone(), traces);
    let mut requests = Requests::new(&node, qos, trace.clone()).await?;
    statuses
        .publish(&Status {
            time: node.clock().now(),
            state: State::Initialized,
        })
        .await?;
    let snapshot = parameters.snapshot();
    let runtime = Runtime::new(
        snapshot.typed.clone(),
        node.clock().clone(),
        controller,
        format!(
            "motion_inference revision {}: {}",
            snapshot.revision, snapshot.effective
        ),
    );
    tokio::select! {
        result = runtime.run(
            sensors,
            limits,
            walking_velocity_limits,
            &mut requests,
            &statuses,
            &timings,
            &trace,
        ) => result,
        () = trace.publish(records) => Ok(()),
    }
}

#[derive(Clone)]
struct SensorLifetime {
    origin: PublicationOrigin,
    expires_at: Time,
}

impl SensorLifetime {
    fn new(origin: PublicationOrigin, maximum_age: Duration) -> Self {
        let expires_at = origin.source_time + maximum_age;
        Self { origin, expires_at }
    }

    fn check(&self, now: Time) -> InferenceResult<()> {
        let reason =
            if self.origin.source_time > self.origin.received_at || self.origin.source_time > now {
                "has a future source timestamp"
            } else if now >= self.expires_at {
                "expired"
            } else {
                return Ok(());
            };
        Err(InferenceError::InputUnavailable {
            input: self.origin.topic.clone(),
            reason: format!("{} {reason}", self.origin.publication_id),
        })
    }
}

type WorkerResult = (
    Inference,
    InferenceResult<Box<Joints<MotorCommand>>>,
    Policy,
    Duration,
);

struct ActiveRequest {
    pending: Pending,
    started_at: Time,
    sensor: SensorLifetime,
    policy_deadline: Option<Time>,
}

struct Runtime {
    parameters: Arc<Parameters>,
    clock: ros_z::time::Clock,
    controller: Option<Inference>,
    worker: Option<tokio::task::JoinHandle<WorkerResult>>,
    active: Option<ActiveRequest>,
    sensor: Option<(SensorFrame, SensorLifetime)>,
    velocity_sensor: Option<String>,
    joint_limits: Option<(Arc<JointLimits>, PublicationOrigin)>,
    walking_velocity_limits: Option<WalkingVelocityLimits>,
    velocity: VelocityEstimator,
    previous_velocity_sensor: Option<String>,
    last_position: Option<(Policy, Joints<f32>)>,
    last_position_request: Option<RequestId>,
    policy_deadline: Option<Time>,
    configuration: Arc<String>,
    fault: Option<Arc<Report>>,
}

impl Runtime {
    fn new(
        parameters: Arc<Parameters>,
        clock: ros_z::time::Clock,
        controller: Option<Inference>,
        configuration: String,
    ) -> Self {
        Self {
            parameters,
            clock,
            controller,
            worker: None,
            active: None,
            sensor: None,
            velocity_sensor: None,
            joint_limits: None,
            walking_velocity_limits: None,
            velocity: VelocityEstimator::default(),
            previous_velocity_sensor: None,
            last_position: None,
            last_position_request: None,
            policy_deadline: None,
            configuration: Arc::new(configuration),
            fault: None,
        }
    }

    #[expect(clippy::too_many_arguments)]
    async fn run(
        mut self,
        sensors: Subscriber<LowState>,
        limits: Subscriber<JointLimits>,
        walking_velocity_limits: Subscriber<WalkingVelocityLimits>,
        requests: &mut Requests,
        statuses: &Publisher<Status>,
        timings: &Publisher<Timing>,
        trace: &Trace,
    ) -> Result<()> {
        let mut published_fault: Option<String> = None;
        loop {
            tokio::select! {
                received = sensors.recv_with_metadata() => {
                    match received {
                        Ok(received) => {
                            if let Some(event) = self.receive_sensor(received) {
                                trace.record_lazy(|| event);
                            }
                        }
                        Err(error @ ros_z::Error::Wire(_)) => {
                            self.sensor = None;
                            self.set_fault(error.into());
                        }
                        Err(error) => return Err(error.into()),
                    }
                }
                received = limits.recv_with_metadata() => {
                    match received {
                        Ok(received) => self.receive_limits(received),
                        Err(error @ ros_z::Error::Wire(_)) => {
                            self.joint_limits = None;
                            self.set_fault(error.into());
                        }
                        Err(error) => return Err(error.into()),
                    }
                }
                received = walking_velocity_limits.recv() => {
                    match received {
                        Ok(limits) => self.receive_walking_velocity_limits(limits),
                        Err(error @ ros_z::Error::Wire(_)) => {
                            self.walking_velocity_limits = None;
                            self.set_fault(error.into());
                        }
                        Err(error) => return Err(error.into()),
                    }
                }
                pending = requests.receive(), if self.worker.is_none() => {
                    if let Some(pending) = pending {
                        self.dispatch(pending).await;
                    }
                }
                completed = async { self.worker.as_mut().expect("active worker").await }, if self.worker.is_some() => {
                    self.complete(completed?, timings).await;
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

    fn receive_sensor(&mut self, received: Received<LowState>) -> Option<Event> {
        let now = self.clock.now();
        self.expire_inputs(now);
        let origin = PublicationOrigin::from_received("inputs/low_state", &received, now);
        let sensor = match sensor_frame(&received, received.source_time) {
            Ok(sensor) => sensor,
            Err(error) => {
                self.sensor = None;
                self.set_fault(error);
                return None;
            }
        };
        if let Err(error) = sensor.validate() {
            self.sensor = None;
            self.set_fault(error);
            return None;
        }
        // Freshness gates policy execution, never the estimator's sensor stream.
        let velocity_updated = self.velocity.update(&sensor, &self.parameters);
        if velocity_updated {
            self.previous_velocity_sensor =
                self.velocity_sensor.replace(origin.publication_id.clone());
            self.sensor = Some((
                sensor,
                SensorLifetime::new(origin.clone(), self.parameters.timing.sensor_timeout),
            ));
            self.expire_inputs(now);
            self.recover_inputs();
        }
        Some(Event::Sensor {
            source: origin,
            velocity_updated,
        })
    }

    fn receive_limits(&mut self, limits: Received<JointLimits>) {
        let origin = PublicationOrigin::from_received("joint_limits", &limits, self.clock.now());
        match limits.validate() {
            Ok(()) => self.joint_limits = Some((Arc::new(limits.message), origin)),
            Err(reason) => {
                self.joint_limits = None;
                self.set_fault(Report::msg(reason));
            }
        }
        self.recover_inputs();
    }

    fn receive_walking_velocity_limits(&mut self, limits: WalkingVelocityLimits) {
        match limits.validate() {
            Ok(()) => self.walking_velocity_limits = Some(limits),
            Err(reason) => {
                self.walking_velocity_limits = None;
                self.set_fault(Report::msg(reason));
            }
        }
        self.recover_inputs();
    }

    fn set_fault(&mut self, error: Report) {
        self.fault = Some(Arc::new(error));
        self.reset_execution();
    }

    fn recover_inputs(&mut self) {
        // Keep any fault latched until the worker that observed it has completed.
        if self.worker.is_none() && self.check_inputs(self.clock.now()).is_ok() {
            self.fault = None;
        }
    }

    fn reset_execution(&mut self) {
        if let Some(controller) = &mut self.controller {
            controller.reset();
        }
        self.last_position = None;
        self.last_position_request = None;
        self.policy_deadline = None;
    }

    fn expire_inputs(&mut self, now: Time) {
        // Reset before use or replacement; keeping the snapshot preserves expiry diagnostics.
        if self
            .sensor
            .as_ref()
            .is_some_and(|(_, input)| input.check(now).is_err())
            || self.policy_deadline.is_some_and(|deadline| now >= deadline)
        {
            self.reset_execution();
        }
    }

    fn check_inputs(&self, now: Time) -> InferenceResult<()> {
        let (_, input) = self
            .sensor
            .as_ref()
            .ok_or_else(|| InferenceError::InputUnavailable {
                input: "inputs/low_state".into(),
                reason: "missing".into(),
            })?;
        input.check(now)?;
        if self.joint_limits.is_none() {
            return Err(InferenceError::InputUnavailable {
                input: "joint_limits".into(),
                reason: "missing".into(),
            });
        }
        if self.walking_velocity_limits.is_none() {
            return Err(InferenceError::InputUnavailable {
                input: WALKING_VELOCITY_LIMITS_TOPIC.into(),
                reason: "missing".into(),
            });
        }
        Ok(())
    }

    async fn dispatch(&mut self, pending: Pending) {
        let now = self.clock.now();
        self.expire_inputs(now);
        if pending.expired(now) {
            pending.reject(InferenceError::DeadlineExpired).await;
            return;
        }
        self.recover_inputs();
        if pending.request.reset {
            self.reset_execution();
        }
        if let Some(source) = &self.fault {
            pending
                .reject(InferenceError::Fault {
                    source: source.clone(),
                })
                .await;
            return;
        }
        if let Err(error) = self.check_inputs(self.clock.now()) {
            pending.reject(error).await;
            return;
        }
        let (mut sensor, sensor_lifetime) = self.sensor.clone().expect("validated sensor");
        let (limits, limits_origin) = self.joint_limits.as_ref().expect("validated limits");
        let request = pending.request.command;
        let (last_commanded_position, last_commanded_request) = match self.last_position {
            Some((policy, _))
                if policy.is_locomotion() && matches!(request, InferenceCommand::GetUp(_)) =>
            {
                (sensor.position, None)
            }
            Some((_, position)) => (position, self.last_position_request.clone()),
            None => (sensor.position, self.last_position_request.clone()),
        };
        sensor.last_commanded_position = last_commanded_position;
        let limits = limits.clone();
        let walking_velocity_limits = self
            .walking_velocity_limits
            .expect("validated walking velocity limits");
        let velocity = self.velocity.clone();
        let mut controller = self.controller.take().expect("idle controller");
        let clock = self.clock.clone();
        let trace = pending.trace.with_inputs(InputSources {
            sensor: sensor_lifetime.origin.clone(),
            previous_velocity_sensor: self.previous_velocity_sensor.clone(),
            joint_limits: limits_origin.clone(),
            last_commanded_request,
            configuration: self.configuration.clone(),
        });
        let expires_at = pending.request.deadline;
        self.active = Some(ActiveRequest {
            pending,
            started_at: now,
            sensor: sensor_lifetime.clone(),
            policy_deadline: self.policy_deadline,
        });
        self.worker = Some(tokio::task::spawn_blocking(move || {
            let start = clock.now();
            let result = if start >= expires_at {
                Err(InferenceError::DeadlineExpired)
            } else {
                sensor_lifetime.check(start).and_then(|()| {
                    controller
                        .execute_request(
                            start,
                            &sensor,
                            request,
                            velocity,
                            &limits,
                            walking_velocity_limits,
                            &trace,
                        )
                        .map_err(|error| InferenceError::Fault {
                            source: Arc::new(error),
                        })
                })
            };
            (
                controller,
                result,
                request.policy(),
                clock.now().duration_since(start),
            )
        }));
    }

    async fn complete(&mut self, completed: WorkerResult, timings: &Publisher<Timing>) {
        self.worker = None;
        self.expire_inputs(self.clock.now());
        let active = self.active.take().expect("active request");
        let (controller, result, policy, compute_duration) = completed;
        let now = self.clock.now();
        let result = self.accept_result(controller, result, policy, &active);
        let ActiveRequest {
            pending,
            started_at,
            sensor,
            ..
        } = active;
        pending.trace.record_at(
            now,
            Event::Outcome {
                error: result.as_ref().err().cloned(),
            },
        );
        let timing = Timing {
            request_id: pending.reply.id().clone(),
            policy: pending.request.command.policy(),
            received_at: pending.received_at,
            started_at,
            completed_at: now,
            sensor_time: sensor.origin.source_time,
            compute_duration,
            accepted: result.is_ok(),
            error: result.as_ref().err().map(ToString::to_string),
        };
        let expires_at = pending.request.deadline;
        let reply_sent = pending.reply.respond(result).await;
        if !reply_sent || self.clock.now() >= expires_at {
            self.reset_execution();
        }
        if let Err(error) = timings.publish_if_subscribed(|| async { timing }).await {
            warn!(%error, "failed to publish inference timing");
        }
    }

    fn accept_result(
        &mut self,
        mut controller: Inference,
        result: InferenceResult<Box<Joints<MotorCommand>>>,
        policy: Policy,
        active: &ActiveRequest,
    ) -> InferenceResult<Box<Joints<MotorCommand>>> {
        let now = self.clock.now();
        let result = if active.pending.expired(now) {
            Err(InferenceError::DeadlineExpired)
        } else if let Some(source) = &self.fault {
            Err(InferenceError::Fault {
                source: source.clone(),
            })
        } else if let Err(error) = active.sensor.check(now) {
            Err(error)
        } else if active
            .policy_deadline
            .is_some_and(|deadline| now >= deadline)
        {
            Err(InferenceError::InputUnavailable {
                input: "policy_state".into(),
                reason: format!(
                    "expired after {:?} without a successful update",
                    self.parameters.timing.policy_state_max_age
                ),
            })
        } else {
            result
        };
        if let Err(InferenceError::Fault { source }) = &result {
            self.fault = Some(source.clone());
        }
        if result.is_err() {
            controller.reset();
            self.reset_execution();
        }
        if let Ok(output) = &result {
            self.last_position = Some((
                policy,
                output
                    .as_ref()
                    .into_iter()
                    .map(|joint| joint.position)
                    .collect(),
            ));
            self.last_position_request = Some(active.pending.reply.id().clone());
            self.policy_deadline = Some(now + self.parameters.timing.policy_state_max_age);
        }
        self.controller = Some(controller);
        result
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
        orientation,
        gyro: low_state.imu_state.angular_velocity,
        last_commanded_position: motors.positions(),
    })
}
