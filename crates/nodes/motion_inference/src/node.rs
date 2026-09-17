use std::{collections::VecDeque, future::Future, pin::Pin, sync::Arc};
use types::joint_limits::JointLimits;
use types::motor_command::MotorCommand;

use booster::{JointsMotorState, LowState};
use color_eyre::{Report, Result, eyre::eyre};
use kinematics::joints::{
    Joints,
    body::{BodyJoints, LowerBodyJoints},
};
use nalgebra::UnitQuaternion;
use ros_z::{
    Message, ServiceTypeInfo,
    entity::TypeInfo,
    message::Service,
    parameter::NodeParameters,
    prelude::*,
    qos::{QosDurability, QosHistory},
    service::ServiceReply,
    time::Time,
};
use serde::{Deserialize, Serialize};
use tokio::task::JoinHandle;

use crate::{
    config::Policy,
    inference::{
        GetUpCommand, Inference, InferenceCommand, InferenceOutput, KickCommand, WalkCommand,
    },
    observation::{self, SensorFrame, VelocityEstimator},
};

pub const SENSOR_TOPIC: &str = "inputs/low_state";
pub const GETUP_INFERENCE_SERVICE: &str = "motion_inference/infer_getup";
pub const KICK_INFERENCE_SERVICE: &str = "motion_inference/infer_kick";
pub const WALK_INFERENCE_SERVICE: &str = "motion_inference/infer_walk";
pub const STATUS_TOPIC: &str = "motion_inference/status";

const REQUEST_QUEUE_CAPACITY: usize = 5;

macro_rules! impl_service {
    ($service:ty, $request:ty, $response:ty) => {
        impl Service for $service {
            type Request = $request;
            type Response = $response;
        }

        impl ServiceTypeInfo for $service {
            fn service_type_info() -> TypeInfo {
                let descriptor = ros_z_schema::ServiceDef::new(
                    concat!(module_path!(), "::", stringify!($service)),
                    <$request>::type_name(),
                    <$response>::type_name(),
                )
                .expect("static inference service descriptor is valid");
                let hash =
                    ros_z_schema::compute_hash(&descriptor).expect("static service hash is valid");
                TypeInfo::new(descriptor.type_name.as_str(), hash)
            }
        }
    };
}

pub struct WalkInferenceService;
pub struct KickInferenceService;
pub struct GetUpInferenceService;

impl_service!(
    WalkInferenceService,
    WalkCommand,
    InferenceResult<Box<LowerBodyJoints<MotorCommand>>>
);
impl_service!(
    KickInferenceService,
    KickCommand,
    InferenceResult<Box<LowerBodyJoints<MotorCommand>>>
);
impl_service!(
    GetUpInferenceService,
    GetUpCommand,
    InferenceResult<Box<Joints<MotorCommand>>>
);

pub fn run_boxed(ctx: Arc<Context>) -> Pin<Box<dyn Future<Output = Result<()>> + Send>> {
    Box::pin(run(ctx))
}

async fn run(ctx: Arc<Context>) -> Result<()> {
    let node = ctx.create_node("motion_inference").build().await?;
    let parameters = node.bind_parameter_as::<Parameters>("motion_inference")?;
    parameters.add_validation_hook(|candidate| {
        candidate.validate().map_err(|error| format!("{error:#}"))
    })?;
    let mut runtime = InferenceNode::new(parameters);
    runtime.start_initialization();
    runtime.run(node).await
}

enum QueuedRequest {
    Walk {
        request: WalkCommand,
        reply: ServiceReply<WalkInferenceService>,
    },
    Kick {
        request: KickCommand,
        reply: ServiceReply<KickInferenceService>,
    },
    GetUp {
        request: GetUpCommand,
        reply: ServiceReply<GetUpInferenceService>,
    },
}

impl QueuedRequest {
    fn command(&self) -> InferenceCommand {
        match self {
            Self::Walk { request, .. } => InferenceCommand::Walk(*request),
            Self::Kick { request, .. } => InferenceCommand::Kick(*request),
            Self::GetUp { request, .. } => InferenceCommand::GetUp(*request),
        }
    }

    async fn respond(self, result: InferenceResult<Box<Joints<MotorCommand>>>) {
        match self {
            Self::Walk { reply, .. } => {
                let response =
                    result.map(|joints| Box::new(LowerBodyJoints::from(BodyJoints::from(*joints))));
                let _ = reply.reply_async(&response).await;
            }
            Self::Kick { reply, .. } => {
                let response =
                    result.map(|joints| Box::new(LowerBodyJoints::from(BodyJoints::from(*joints))));
                let _ = reply.reply_async(&response).await;
            }
            Self::GetUp { reply, .. } => {
                let _ = reply.reply_async(&result).await;
            }
        }
    }

    async fn deny(self, error: InferenceError) {
        self.respond(Err(error)).await;
    }
}

#[derive(Clone, Copy)]
enum Job {
    Initialize,
    Inference,
}

enum Completion {
    Initialized,
    Inference(InferenceOutput),
}

struct Worker {
    handle: JoinHandle<(Controller, Result<Completion>)>,
    job: Job,
    request: Option<QueuedRequest>,
    response_sent: bool,
}

struct InferenceNode {
    parameters: NodeParameters<Parameters>,
    joint_limits: Option<Arc<JointLimits>>,
    controller: Option<Controller>,
    worker: Option<Worker>,
    pending: VecDeque<QueuedRequest>,
    first_fault: Option<InferenceError>,
    sensor: Option<SensorFrame>,
    last_inferred_position: Option<Joints<f32>>,
    velocity: VelocityEstimator,
}

impl InferenceNode {
    fn new(parameters: NodeParameters<Parameters>) -> Self {
        Self {
            joint_limits: None,
            controller: Some(Controller::new(parameters.snapshot().typed.clone())),
            parameters,
            worker: None,
            pending: VecDeque::with_capacity(REQUEST_QUEUE_CAPACITY),
            first_fault: None,
            sensor: None,
            last_inferred_position: None,
            velocity: VelocityEstimator::default(),
        }
    }

    fn initialized(&self) -> bool {
        self.controller
            .as_ref()
            .is_some_and(Controller::initialized)
            || self
                .worker
                .as_ref()
                .is_some_and(|worker| matches!(worker.job, Job::Inference))
    }

    fn start_initialization(&mut self) {
        let mut controller = self.controller.take().expect("startup controller exists");
        let handle = tokio::task::spawn_blocking(move || {
            let result = controller.initialize().map(|()| Completion::Initialized);
            (controller, result)
        });
        self.worker = Some(Worker {
            handle,
            job: Job::Initialize,
            request: None,
            response_sent: false,
        });
    }

    async fn run(&mut self, node: Node) -> Result<()> {
        let latest = QosProfile {
            history: QosHistory::from_depth(1),
            ..Default::default()
        };
        let joint_limits = node
            .subscriber::<JointLimits>("joint_limits")
            .qos(QosProfile {
                durability: QosDurability::TransientLocal,
                history: QosHistory::from_depth(1),
                ..Default::default()
            })
            .build()
            .await?;
        let sensors = node
            .subscriber::<LowState>(SENSOR_TOPIC)
            .qos(latest)
            .build()
            .await?;
        let mut getup_requests = node
            .service_server::<GetUpInferenceService>(GETUP_INFERENCE_SERVICE)
            .qos(QosProfile {
                history: QosHistory::KeepAll,
                ..Default::default()
            })
            .build()
            .await?;
        let mut kick_requests = node
            .service_server::<KickInferenceService>(KICK_INFERENCE_SERVICE)
            .qos(QosProfile {
                history: QosHistory::KeepAll,
                ..Default::default()
            })
            .build()
            .await?;
        let mut walk_requests = node
            .service_server::<WalkInferenceService>(WALK_INFERENCE_SERVICE)
            .qos(QosProfile {
                history: QosHistory::KeepAll,
                ..Default::default()
            })
            .build()
            .await?;
        let statuses = node
            .publisher::<Status>(STATUS_TOPIC)
            .qos(QosProfile {
                durability: QosDurability::TransientLocal,
                ..Default::default()
            })
            .build()
            .await?;
        statuses
            .publish(&Status {
                time: node.clock().now(),
                state: State::Idle,
            })
            .await?;

        loop {
            if let Some(error) = self.first_fault.clone() {
                if let Some(worker) = &mut self.worker
                    && !worker.response_sent
                {
                    worker.response_sent = true;
                    if let Some(request) = worker.request.take() {
                        request.deny(error.clone()).await;
                    }
                }
                self.reject_pending(&error).await;
            }
            tokio::select! {
                completed = async { (&mut self.worker.as_mut().expect("worker exists").handle).await }, if self.worker.is_some() => {
                    let mut worker = self.worker.take().expect("worker completed");
                    let (controller, result) = match completed {
                        Ok(completed) => completed,
                        Err(error) => {
                            let error = InferenceError::WorkerFailed { source: Arc::new(Report::new(error)) };
                            if !worker.response_sent {
                                self.fail_job(&node, &statuses, worker.request.take(), error.clone()).await?;
                            }
                            self.reject_pending(&error).await;
                            return Err(error.into());
                        }
                    };
                    self.controller = Some(controller);
                    if worker.response_sent { continue; }
                    match result {
                        Ok(Completion::Initialized) => {
                            self.sensor = None;
                            self.last_inferred_position = None;
                            self.velocity = VelocityEstimator::default();
                            statuses.publish(&Status { time: node.clock().now(), state: State::Initialized }).await?;
                        }
                        Ok(Completion::Inference(output)) => {
                            self.last_inferred_position = Some(output.joints.as_ref().into_iter().map(|joint| joint.position).collect());
                            worker.request.take().expect("active request has a reply").respond(Ok(output.joints)).await;
                        }
                        Err(error) => {
                            let source = Arc::new(error);
                            let error = match worker.job {
                                Job::Initialize => InferenceError::InitializationFailed { source },
                                Job::Inference => InferenceError::InferenceFailed { source },
                            };
                            self.fail_job(&node, &statuses, worker.request.take(), error).await?;
                        }
                    }
                }
                received = joint_limits.recv() => {
                    let received = received?;
                    match received.validate() {
                        Ok(()) => self.joint_limits = Some(Arc::new(received)),
                        Err(reason) => statuses.publish(&fault_status(&mut self.first_fault, node.clock().now(), InferenceError::InvalidJointLimits { source: Arc::new(Report::msg(reason)) })).await?,
                    }
                }
                received = sensors.recv_with_metadata() => {
                    let received = received?;
                    if self.initialized() && self.first_fault.is_none() {
                        let parameters = self.parameters.snapshot();
                        let result = sensor_frame(&received.message, received.source_time).and_then(|sensor| {
                            sensor.validate(parameters.typed())?;
                            self.velocity.update(&sensor, parameters.typed())?;
                            Ok(sensor)
                        });
                        match result {
                            Ok(sensor) => self.sensor = Some(sensor),
                            Err(error) => statuses.publish(&fault_status(&mut self.first_fault, node.clock().now(), InferenceError::InvalidSensorFrame { source: Arc::new(error) })).await?,
                        }
                    }
                }
                received = getup_requests.take_request_async() => {
                    let (request, reply) = received?.into_parts();
                    self.enqueue(QueuedRequest::GetUp { request, reply }).await;
                }
                received = kick_requests.take_request_async() => {
                    let (request, reply) = received?.into_parts();
                    self.enqueue(QueuedRequest::Kick { request, reply }).await;
                }
                received = walk_requests.take_request_async() => {
                    let (request, reply) = received?.into_parts();
                    self.enqueue(QueuedRequest::Walk { request, reply }).await;
                }
                () = std::future::ready(()), if self.worker.is_none() && !self.pending.is_empty() => {
                    let queued = self.pending.pop_front().expect("queued request exists");
                    let request = queued.command();
                    let Some(joints) = self.joint_limits.clone() else {
                        queued.deny(InferenceError::MissingJointLimits).await;
                        continue;
                    };
                    let Some(sensor) = &self.sensor else {
                        queued.deny(InferenceError::MissingSensorFrame).await;
                        continue;
                    };
                    let mut sensor = sensor.clone();
                    sensor.last_commanded_position = self.last_inferred_position.unwrap_or(sensor.position);
                    let parameters = self.parameters.snapshot().typed.clone();
                    if let Err(error) = sensor.validate(&parameters) {
                        self.fail_job(&node, &statuses, Some(queued), InferenceError::InvalidSensorFrame { source: Arc::new(error) }).await?;
                        continue;
                    }
                    let velocity = self.velocity.clone();
                    let mut controller = self.controller.take().expect("idle controller exists");
                    let clock = node.clock().clone();
                    let handle = tokio::task::spawn_blocking(move || {
                        let result = controller.execute(clock.now(), &sensor, request, velocity, &joints, parameters)
                            .map(Completion::Inference);
                        (controller, result)
                    });
                    self.worker = Some(Worker { handle, job: Job::Inference, request: Some(queued), response_sent: false });
                }
            }
        }
    }

    async fn enqueue(&mut self, request: QueuedRequest) {
        if let Some(error) = &self.first_fault {
            request.deny(error.clone()).await;
        } else if !self.initialized() {
            request.deny(InferenceError::Initializing).await;
        } else if let Some(oldest) = enqueue_request(&mut self.pending, request) {
            oldest.deny(InferenceError::SorryQueueVol).await;
        }
    }

    async fn reject_pending(&mut self, error: &InferenceError) {
        while let Some(queued) = self.pending.pop_front() {
            queued.deny(error.clone()).await;
        }
    }

    async fn fail_job(
        &mut self,
        node: &Node,
        statuses: &Publisher<Status>,
        request: Option<QueuedRequest>,
        error: InferenceError,
    ) -> Result<()> {
        statuses
            .publish(&fault_status(
                &mut self.first_fault,
                node.clock().now(),
                error.clone(),
            ))
            .await?;
        if let Some(request) = request {
            request.deny(error).await;
        }
        Ok(())
    }
}

fn enqueue_request<T>(pending: &mut VecDeque<T>, request: T) -> Option<T> {
    let denied = if pending.len() == REQUEST_QUEUE_CAPACITY {
        pending.pop_front()
    } else {
        None
    };
    pending.push_back(request);
    denied
}

fn fault_status(
    first_fault: &mut Option<InferenceError>,
    time: Time,
    error: InferenceError,
) -> Status {
    let reason = first_fault.get_or_insert(error).to_string();
    Status {
        time,
        state: State::Fault { reason },
    }
}

mod messages {
    use std::result::Result;
    pub type InferenceResult<T> = Result<T, InferenceError>;

    use super::*;
    /// Shared sources keep the native cause intact when a fault rejects multiple requests.
    #[derive(Clone, Debug, Serialize, Deserialize, Message, thiserror::Error)]
    pub enum InferenceError {
        #[error("cannot serve motion inference requests while policy models are initializing")]
        Initializing,
        #[error(
            "motion inference request queue is full (capacity: {}); the oldest pending request was denied to accept a newer request",
            REQUEST_QUEUE_CAPACITY
        )]
        SorryQueueVol,
        #[error("cannot run motion inference: global joint limits have not been received")]
        MissingJointLimits,
        #[error("cannot run motion inference: no valid sensor frame has been received")]
        MissingSensorFrame,
        #[error("received invalid global joint limits for motion inference: {source:#}")]
        InvalidJointLimits {
            #[source]
            #[serde(with = "ros_z::message::report")]
            source: Arc<Report>,
        },
        #[error("failed to process a motion inference sensor frame: {source:#}")]
        InvalidSensorFrame {
            #[source]
            #[serde(with = "ros_z::message::report")]
            source: Arc<Report>,
        },
        #[error("failed to initialize motion inference policy models: {source:#}")]
        InitializationFailed {
            #[source]
            #[serde(with = "ros_z::message::report")]
            source: Arc<Report>,
        },
        #[error("failed to execute the motion inference request: {source:#}")]
        InferenceFailed {
            #[source]
            #[serde(with = "ros_z::message::report")]
            source: Arc<Report>,
        },
        #[error("motion inference worker task failed: {source:#}")]
        WorkerFailed {
            #[source]
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
}

pub use crate::config::Parameters;
pub use messages::{InferenceError, InferenceResult, State, Status};

fn sensor_frame(low_state: &LowState, timestamp: Time) -> Result<observation::SensorFrame> {
    let motors = low_state.serial_motor_states()?;
    let angles = low_state.imu_state.roll_pitch_yaw;
    let orientation = UnitQuaternion::from_euler_angles(angles.x(), angles.y(), angles.z());
    let position = motors.positions();
    Ok(observation::SensorFrame {
        timestamp,
        position,
        velocity: motors.velocities(),
        orientation: orientation.into_inner(),
        gyro: low_state.imu_state.angular_velocity,
        last_commanded_position: position,
    })
}

struct Controller {
    startup_parameters: Arc<Parameters>,
    inference: Option<Inference>,
}

impl Controller {
    fn new(parameters: Arc<Parameters>) -> Self {
        Self {
            startup_parameters: parameters,
            inference: None,
        }
    }

    fn initialized(&self) -> bool {
        self.inference.is_some()
    }

    fn initialize(&mut self) -> Result<()> {
        self.inference = Some(Inference::new(
            &self.startup_parameters.neural_networks_folder,
            &Policy::ALL,
            self.startup_parameters.clone(),
        )?);
        Ok(())
    }

    fn execute(
        &mut self,
        now: Time,
        sensor: &SensorFrame,
        command: InferenceCommand,
        velocity: VelocityEstimator,
        joints: &JointLimits,
        parameters: Arc<Parameters>,
    ) -> Result<InferenceOutput> {
        let inference = self
            .inference
            .as_mut()
            .ok_or_else(|| eyre!("inference policy models have not been initialized"))?;
        inference.execute_request(now, sensor, command, velocity, joints, parameters)
    }
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

    #[test]
    fn request_queue_is_fifo_up_to_five_pending_requests() {
        let mut pending = VecDeque::new();
        for request in 0..5 {
            assert_eq!(enqueue_request(&mut pending, request), None);
        }
        for request in 0..5 {
            assert_eq!(pending.pop_front(), Some(request));
        }
        assert!(pending.is_empty());
    }

    #[test]
    fn full_request_queue_denies_oldest_and_retains_newest_five() {
        let mut pending = VecDeque::new();
        for request in 0..5 {
            assert_eq!(enqueue_request(&mut pending, request), None);
        }
        for request in 5..10 {
            assert_eq!(enqueue_request(&mut pending, request), Some(request - 5));
            assert_eq!(pending.len(), 5);
        }
        assert_eq!(pending, VecDeque::from([5, 6, 7, 8, 9]));

        assert_eq!(pending.pop_front(), Some(5));
        assert_eq!(enqueue_request(&mut pending, 10), None);
        assert_eq!(pending, VecDeque::from([6, 7, 8, 9, 10]));
    }
}
