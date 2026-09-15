use std::{collections::VecDeque, future::Future, pin::Pin, sync::Arc};
use types::joint_limits::JointLimits;

use anyhow::anyhow;
use booster::{JointsMotorState, LowState};
use color_eyre::Result;
use kinematics::joints::Joints;
use nalgebra::UnitQuaternion;
use ros_z::{
    Message, ServiceTypeInfo,
    entity::TypeInfo,
    message::Service,
    prelude::*,
    qos::{QosDurability, QosHistory},
    service::ServiceReply,
    time::Time,
};
use serde::{Deserialize, Serialize};
use tokio::task::JoinHandle;

use crate::{
    config::Policy,
    inference::{Inference, InferenceCommand, InferenceOutput},
    observation::{self, SensorFrame, VelocityEstimator},
};

pub const SENSOR_TOPIC: &str = "inputs/low_state";
pub const INFERENCE_SERVICE: &str = "motion_inference/infer";
pub const STATUS_TOPIC: &str = "motion_inference/status";

const REQUEST_QUEUE_CAPACITY: usize = 5;

pub struct Infer;

impl Service for Infer {
    type Request = Request;
    type Response = Response;
}

impl ServiceTypeInfo for Infer {
    fn service_type_info() -> TypeInfo {
        let descriptor = ros_z_schema::ServiceDef::new(
            "motion_inference::node::Infer",
            Request::type_name(),
            Response::type_name(),
        )
        .expect("static inference service descriptor is valid");
        let hash = ros_z_schema::compute_hash(&descriptor).expect("static service hash is valid");
        TypeInfo::new(descriptor.type_name.as_str(), hash)
    }
}

pub fn run_boxed(ctx: Arc<Context>) -> Pin<Box<dyn Future<Output = Result<()>> + Send>> {
    Box::pin(run(ctx))
}

async fn run(ctx: Arc<Context>) -> Result<()> {
    let node = ctx.create_node("motion_inference").build().await?;
    let parameters = node.bind_parameter_as::<Parameters>("motion_inference")?;
    let snapshot = Arc::new(parameters.snapshot().typed().clone());
    snapshot
        .validate()
        .map_err(|error| color_eyre::eyre::eyre!("{error:#}"))?;
    let frozen = snapshot.clone();
    parameters.add_validation_hook(move |candidate| {
        if candidate != frozen.as_ref() {
            return Err("motion inference parameters are startup-only; edit startup configuration and restart the process".into());
        }
        Ok(())
    })?;
    let mut runtime = InferenceNode::new(snapshot);
    runtime.start_initialization();
    runtime.run(node).await
}

struct QueuedRequest {
    request: Request,
    reply: ServiceReply<Infer>,
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
    handle: JoinHandle<(Controller, anyhow::Result<Completion>)>,
    job: Job,
    reply: Option<ServiceReply<Infer>>,
    response_sent: bool,
}

struct InferenceNode {
    parameters: Arc<Parameters>,
    joint_limits: Option<Arc<JointLimits>>,
    controller: Option<Controller>,
    worker: Option<Worker>,
    pending: VecDeque<QueuedRequest>,
    first_fault: Option<String>,
    sensor: Option<SensorFrame>,
    last_inferred_position: Option<Joints<f32>>,
    velocity: VelocityEstimator,
}

impl InferenceNode {
    fn new(parameters: Arc<Parameters>) -> Self {
        Self {
            joint_limits: None,
            controller: Some(Controller::new(parameters.clone())),
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
            reply: None,
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
        let mut requests = node
            .service_server::<Infer>(INFERENCE_SERVICE)
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
            if let Some(reason) = self.first_fault.clone() {
                if let Some(worker) = &mut self.worker
                    && !worker.response_sent
                {
                    worker.response_sent = true;
                    if let Some(reply) = worker.reply.take() {
                        deny(reply, &reason).await;
                    }
                }
                self.reject_pending(&reason).await;
            }
            tokio::select! {
                completed = async { (&mut self.worker.as_mut().expect("worker exists").handle).await }, if self.worker.is_some() => {
                    let mut worker = self.worker.take().expect("worker completed");
                    let (controller, result) = match completed {
                        Ok(completed) => completed,
                        Err(error) => {
                            let reason = format!("motion inference worker failed: {error}");
                            if !worker.response_sent {
                                self.fail_job(&node, &statuses, worker.reply.take(), &reason).await?;
                            }
                            self.reject_pending(&reason).await;
                            return Err(color_eyre::eyre::eyre!(reason));
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
                            self.last_inferred_position = Some((*output.joints).into_iter().map(|joint| joint.position).collect());
                            respond(worker.reply.take().expect("active request has a reply"), Ok(output)).await;
                        }
                        Err(error) => self.fail_job(&node, &statuses, worker.reply.take(), &format!("{error:#}")).await?,
                    }
                }
                received = joint_limits.recv() => {
                    let received = received?;
                    match received.validate() {
                        Ok(()) => self.joint_limits = Some(Arc::new(received)),
                        Err(reason) => statuses.publish(&fault_status(&mut self.first_fault, node.clock().now(), anyhow!(reason))).await?,
                    }
                }
                received = sensors.recv_with_metadata() => {
                    let received = received?;
                    if self.initialized() && self.first_fault.is_none() {
                        let result = sensor_frame(&received.message, received.source_time).and_then(|sensor| {
                            sensor.validate(&self.parameters)?;
                            self.velocity.update(&sensor, &self.parameters)?;
                            Ok(sensor)
                        });
                        match result {
                            Ok(sensor) => self.sensor = Some(sensor),
                            Err(error) => statuses.publish(&fault_status(&mut self.first_fault, node.clock().now(), error)).await?,
                        }
                    }
                }
                received = requests.take_request_async() => {
                    let (request, reply) = received?.into_parts();
                    if let Some(reason) = &self.first_fault {
                        deny(reply, reason).await;
                    } else if !self.initialized() {
                        deny(reply, "inference is initializing").await;
                    } else if let Some(oldest) = enqueue_request(&mut self.pending, QueuedRequest { request, reply }) {
                        deny(oldest.reply, "inference request queue is full").await;
                    }
                }
                () = std::future::ready(()), if self.worker.is_none() && !self.pending.is_empty() => {
                    let queued = self.pending.pop_front().expect("queued request exists");
                    let request = queued.request;
                    let Some(joints) = self.joint_limits.clone() else {
                        deny(queued.reply, "global joint limits are missing").await;
                        continue;
                    };
                    let Some(sensor) = &self.sensor else {
                        deny(queued.reply, "sensor frame is missing").await;
                        continue;
                    };
                    let mut sensor = sensor.clone();
                    sensor.last_commanded_position = self.last_inferred_position.unwrap_or(sensor.position);
                    if let Err(error) = sensor.validate(&self.parameters) {
                        let reason = format!("{error:#}");
                        statuses.publish(&fault_status(&mut self.first_fault, node.clock().now(), error)).await?;
                        deny(queued.reply, &reason).await;
                        continue;
                    }
                    let velocity = self.velocity.clone();
                    let mut controller = self.controller.take().expect("idle controller exists");
                    let clock = node.clock().clone();
                    let handle = tokio::task::spawn_blocking(move || {
                        let result = controller.execute(clock.now(), &sensor, request, velocity, &joints)
                            .map(Completion::Inference);
                        (controller, result)
                    });
                    self.worker = Some(Worker { handle, job: Job::Inference, reply: Some(queued.reply), response_sent: false });
                }
            }
        }
    }

    async fn reject_pending(&mut self, reason: &str) {
        while let Some(queued) = self.pending.pop_front() {
            deny(queued.reply, reason).await;
        }
    }

    async fn fail_job(
        &mut self,
        node: &Node,
        statuses: &Publisher<Status>,
        reply: Option<ServiceReply<Infer>>,
        reason: &str,
    ) -> Result<()> {
        statuses
            .publish(&fault_status(
                &mut self.first_fault,
                node.clock().now(),
                anyhow!("{reason}"),
            ))
            .await?;
        if let Some(reply) = reply {
            deny(reply, reason).await;
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

async fn respond(reply: ServiceReply<Infer>, result: InferenceResult) {
    let _ = reply.reply_async(&result).await;
}

async fn deny(reply: ServiceReply<Infer>, reason: &str) {
    respond(
        reply,
        Err(InferenceError {
            reason: reason.to_owned(),
        }),
    )
    .await;
}

fn fault_status(first_fault: &mut Option<String>, time: Time, error: anyhow::Error) -> Status {
    let reason = first_fault
        .get_or_insert_with(|| format!("{error:#}"))
        .clone();
    Status {
        time,
        state: State::Fault { reason },
    }
}

mod messages {
    use super::*;

    pub type Request = InferenceCommand;
    pub type Response = InferenceResult;
    pub type InferenceResult = std::result::Result<InferenceOutput, InferenceError>;

    #[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, Message)]
    pub struct InferenceError {
        pub reason: String,
    }

    impl std::fmt::Display for InferenceError {
        fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            formatter.write_str(&self.reason)
        }
    }

    impl std::error::Error for InferenceError {}

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
pub use messages::{InferenceError, InferenceResult, Request, Response, State, Status};

fn sensor_frame(low_state: &LowState, timestamp: Time) -> anyhow::Result<observation::SensorFrame> {
    let motors = low_state
        .serial_motor_states()
        .map_err(|error| anyhow!("{error:#}"))?;
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
    parameters: Arc<Parameters>,
    inference: Option<Inference>,
}

impl Controller {
    fn new(parameters: Arc<Parameters>) -> Self {
        Self {
            parameters,
            inference: None,
        }
    }

    fn initialized(&self) -> bool {
        self.inference.is_some()
    }

    fn initialize(&mut self) -> anyhow::Result<()> {
        self.inference = Some(Inference::new(
            &self.parameters.neural_networks_folder,
            &Policy::ALL,
            self.parameters.clone(),
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
    ) -> anyhow::Result<InferenceOutput> {
        let inference = self
            .inference
            .as_mut()
            .ok_or_else(|| anyhow!("inference is initializing"))?;
        inference.execute_request(now, sensor, command, velocity, joints)
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
