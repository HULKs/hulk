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
    pubsub::Received,
    qos::{QosDurability, QosHistory, QosReliability},
    service::RequestId,
    time::Time,
};
use serde::{Deserialize, Serialize};
use tracing::warn;
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
    /// Wall-clock deadline: service timeouts must still expire when logical time is paused.
    pub deadline: Time,
}

impl<T> InferenceRequest<T> {
    pub fn new(command: T, reset: bool, timeout: Duration) -> Self {
        Self {
            command,
            reset,
            deadline: Time::from_wallclock(std::time::SystemTime::now()) + timeout,
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

const SENSOR_TIMEOUT: Duration = Duration::from_millis(40);
const POLICY_STATE_MAX_AGE: Duration = Duration::from_millis(200);

#[derive(Clone)]
struct SensorLifetime {
    origin: PublicationOrigin,
    received: Instant,
    age_at_receipt: Duration,
    maximum_age: Duration,
}

impl SensorLifetime {
    fn new(origin: PublicationOrigin, received: Instant, maximum_age: Duration) -> Self {
        let age_at_receipt = origin.received_at.duration_since(origin.source_time);
        Self {
            origin,
            received,
            age_at_receipt,
            maximum_age,
        }
    }

    fn deadline(&self) -> Instant {
        self.received + self.maximum_age.saturating_sub(self.age_at_receipt)
    }

    fn check(&self, now: Instant, source_clock_now: Time) -> InferenceResult<()> {
        if self.origin.source_time > self.origin.received_at
            || self.origin.source_time > source_clock_now
        {
            return Err(InferenceError::InputUnavailable {
                input: self.origin.topic.clone(),
                reason: format!(
                    "{} has a future source timestamp",
                    self.origin.publication_id
                ),
            });
        }
        // Transport delay consumes TTL; clock adjustments cannot rejuvenate a sample.
        let age = (self.age_at_receipt + now.saturating_duration_since(self.received))
            .max(source_clock_now.duration_since(self.origin.source_time));
        if age >= self.maximum_age {
            return Err(InferenceError::InputUnavailable {
                input: self.origin.topic.clone(),
                reason: format!(
                    "{} expired: age {age:?}, limit {:?}",
                    self.origin.publication_id, self.maximum_age
                ),
            });
        }
        Ok(())
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
    policy_deadline: Option<Instant>,
}

struct Runtime {
    parameters: Arc<Parameters>,
    clock: ros_z::time::Clock,
    controller: Option<Inference>,
    worker: Option<tokio::task::JoinHandle<WorkerResult>>,
    active: Option<ActiveRequest>,
    sensor: Option<SensorFrame>,
    sensor_lifetime: Option<SensorLifetime>,
    joint_limits: Option<(Arc<JointLimits>, PublicationOrigin)>,
    walking_velocity_limits: Option<WalkingVelocityLimits>,
    velocity: VelocityEstimator,
    previous_velocity_sensor: Option<String>,
    last_position: Option<(Policy, Joints<f32>)>,
    last_position_request: Option<RequestId>,
    policy_deadline: Option<Instant>,
    configuration: String,
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
            sensor_lifetime: None,
            joint_limits: None,
            walking_velocity_limits: None,
            velocity: VelocityEstimator::default(),
            previous_velocity_sensor: None,
            last_position: None,
            last_position_request: None,
            policy_deadline: None,
            configuration,
            fault: None,
        }
    }

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
            self.expire_inputs(Instant::now());
            let expiry = self.next_expiry();
            tokio::select! {
                _ = async {
                    match expiry {
                        Some(deadline) => tokio::time::sleep_until(deadline.into()).await,
                        None => std::future::pending().await,
                    }
                } => {}
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
                        self.expire_inputs(Instant::now());
                        self.dispatch(pending).await;
                    }
                }
                completed = async { self.worker.as_mut().expect("active worker").await }, if self.worker.is_some() => {
                    self.complete(completed?, timings).await;
                }
            }
            self.expire_inputs(Instant::now());
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
        let now = Instant::now();
        self.expire_inputs(now);
        let origin =
            PublicationOrigin::from_received("inputs/low_state", &received, self.clock.now());
        let sensor = match sensor_frame(&received, received.source_time) {
            Ok(sensor) => sensor,
            Err(error) => {
                self.sensor = None;
                self.set_fault(error);
                return None;
            }
        };
        if let Err(error) = sensor.validate(&self.parameters) {
            self.sensor = None;
            self.set_fault(error);
            return None;
        }
        // Freshness gates policy execution, never the estimator's sensor stream.
        let velocity_updated = self.velocity.update(&sensor, &self.parameters);
        if velocity_updated {
            self.previous_velocity_sensor = self
                .sensor_lifetime
                .as_ref()
                .map(|input| input.origin.publication_id.clone());
            self.sensor_lifetime = Some(SensorLifetime::new(origin.clone(), now, SENSOR_TIMEOUT));
            self.sensor = Some(sensor);
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
        if self.worker.is_none()
            && self.sensor.is_some()
            && self.joint_limits.is_some()
            && self.walking_velocity_limits.is_some()
        {
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

    fn next_expiry(&self) -> Option<Instant> {
        let sensor_deadline = self
            .sensor
            .as_ref()
            .and(self.sensor_lifetime.as_ref())
            .map(SensorLifetime::deadline);
        sensor_deadline
            .into_iter()
            .chain(self.policy_deadline)
            .min()
    }

    fn expire_inputs(&mut self, now: Instant) {
        let sensor_expired = self.sensor.is_some()
            && self
                .sensor_lifetime
                .as_ref()
                .is_some_and(|input| input.check(now, self.clock.now()).is_err());
        if sensor_expired {
            self.sensor = None;
        }
        if sensor_expired || self.policy_deadline.is_some_and(|deadline| now >= deadline) {
            self.reset_execution();
        }
    }

    fn check_inputs(&self, now: Instant) -> InferenceResult<()> {
        if let Some(input) = &self.sensor_lifetime {
            input.check(now, self.clock.now())?;
        }
        if self.sensor.is_none() {
            return Err(InferenceError::InputUnavailable {
                input: "inputs/low_state".into(),
                reason: "missing".into(),
            });
        }
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
        self.expire_inputs(Instant::now());
        let now = self.clock.now();
        if pending.expired() {
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
        if let Err(error) = self.check_inputs(Instant::now()) {
            pending.reject(error).await;
            return;
        }
        let mut sensor = self.sensor.clone().expect("validated sensor");
        let (limits, limits_origin) = self.joint_limits.as_ref().expect("validated limits");
        let sensor_lifetime = self
            .sensor_lifetime
            .clone()
            .expect("validated sensor origin");
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
        let parameters = self.parameters.clone();
        let trace = pending.trace.with_inputs(InputSources {
            sensor: sensor_lifetime.origin.clone(),
            previous_velocity_sensor: self.previous_velocity_sensor.clone(),
            joint_limits: limits_origin.clone(),
            last_commanded_request,
            configuration: self.configuration.clone(),
        });
        let expires_at = pending.expires_at;
        self.active = Some(ActiveRequest {
            pending,
            started_at: now,
            sensor: sensor_lifetime.clone(),
            policy_deadline: self.policy_deadline,
        });
        self.worker = Some(tokio::task::spawn_blocking(move || {
            let start = Instant::now();
            let result = if start >= expires_at {
                Err(InferenceError::DeadlineExpired)
            } else {
                sensor_lifetime.check(start, clock.now()).and_then(|()| {
                    let now = clock.now();
                    controller.update_parameters(parameters);
                    controller
                        .execute_request(
                            now,
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
            (controller, result, request.policy(), start.elapsed())
        }));
    }

    async fn complete(&mut self, completed: WorkerResult, timings: &Publisher<Timing>) {
        self.worker = None;
        self.expire_inputs(Instant::now());
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
        let expires_at = pending.expires_at;
        let reply_sent = pending.reply.respond(result).await;
        if !reply_sent || Instant::now() >= expires_at {
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
        let now = Instant::now();
        let result = if now >= active.pending.expires_at {
            Err(InferenceError::DeadlineExpired)
        } else if let Some(source) = &self.fault {
            Err(InferenceError::Fault {
                source: source.clone(),
            })
        } else if let Err(error) = active.sensor.check(now, self.clock.now()) {
            Err(error)
        } else if active
            .policy_deadline
            .is_some_and(|deadline| now >= deadline)
        {
            Err(InferenceError::InputUnavailable {
                input: "policy_state".into(),
                reason: format!(
                    "expired after {POLICY_STATE_MAX_AGE:?} without a successful update"
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
            self.policy_deadline = Some(now + POLICY_STATE_MAX_AGE);
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
        orientation: orientation.into_inner(),
        gyro: low_state.imu_state.angular_velocity,
        last_commanded_position: motors.positions(),
    })
}
