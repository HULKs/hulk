use std::time::{Instant, SystemTime};

use crate::{
    inference::InferenceCommand,
    node::{
        GETUP_INFERENCE_SERVICE, GetUpInferenceService, InferenceError, InferenceRequest,
        InferenceResult, KICK_INFERENCE_SERVICE, KickInferenceService, WALK_INFERENCE_SERVICE,
        WalkInferenceService,
    },
    trace::{Event, Trace},
};
use kinematics::joints::{
    Joints,
    body::{BodyJoints, LowerBodyJoints},
};
use ros_z::{
    prelude::*,
    qos::QosHistory,
    service::{RequestId, ServiceReply, ServiceServer},
    time::{Clock, Time},
};
use tracing::warn;
use types::motor_command::MotorCommand;

pub(super) struct Pending {
    pub request: InferenceRequest<InferenceCommand>,
    pub reply: InferenceReply,
    pub received_at: Time,
    pub trace: Trace,
    pub expires_at: Instant,
}

impl Pending {
    pub fn expired(&self) -> bool {
        Instant::now() >= self.expires_at
    }

    pub(super) async fn reject(self, error: InferenceError) {
        self.trace.record(Event::Outcome {
            error: Some(error.clone()),
        });
        self.reply.respond(Err(error)).await;
    }
}

fn request_expiry(deadline: Time, wall_now: Time, now: Instant) -> Instant {
    now.checked_add(deadline.duration_since(wall_now))
        .unwrap_or(now)
}

pub(super) struct Requests {
    walk: ServiceServer<WalkInferenceService>,
    kick: ServiceServer<KickInferenceService>,
    get_up: ServiceServer<GetUpInferenceService>,
    clock: Clock,
    trace: Trace,
}

impl Requests {
    pub async fn new(node: &Node, qos: QosProfile, trace: Trace) -> ros_z::Result<Self> {
        let qos = QosProfile {
            history: QosHistory::from_depth(2),
            ..qos
        };
        Ok(Self {
            walk: node
                .service_server::<WalkInferenceService>(WALK_INFERENCE_SERVICE)
                .qos(qos)
                .build()
                .await?,
            kick: node
                .service_server::<KickInferenceService>(KICK_INFERENCE_SERVICE)
                .qos(qos)
                .build()
                .await?,
            get_up: node
                .service_server::<GetUpInferenceService>(GETUP_INFERENCE_SERVICE)
                .qos(qos)
                .build()
                .await?,
            clock: node.clock().clone(),
            trace,
        })
    }

    pub async fn receive(&mut self) -> Option<Pending> {
        let (service, received) = tokio::select! {
            received = self.walk.take_request_async() => (
                WALK_INFERENCE_SERVICE,
                received.map(|received| {
                    let (request, reply) = received.into_parts();
                    (request.map(InferenceCommand::Walk), InferenceReply::Walk(reply))
                }),
            ),
            received = self.kick.take_request_async() => (
                KICK_INFERENCE_SERVICE,
                received.map(|received| {
                    let (request, reply) = received.into_parts();
                    (request.map(InferenceCommand::Kick), InferenceReply::Kick(reply))
                }),
            ),
            received = self.get_up.take_request_async() => (
                GETUP_INFERENCE_SERVICE,
                received.map(|received| {
                    let (request, reply) = received.into_parts();
                    (request.map(InferenceCommand::GetUp), InferenceReply::GetUp(reply))
                }),
            ),
        };
        // Native service queues expose no arrival timestamp; this records dequeue time.
        let received_at = self.clock.now();
        let (request, reply) = match received {
            Ok(received) => received,
            Err(error) => {
                warn!(service, %error, "discarding malformed inference request");
                self.trace.record_at(
                    received_at,
                    Event::DecodeError {
                        service: service.to_owned(),
                        error: error.to_string(),
                    },
                );
                return None;
            }
        };
        let trace = self.trace.for_request(reply.id().clone());
        trace.record_at(received_at, Event::Request(request.command));
        // Sample monotonic time first: preemption between clock reads must
        // shorten the deadline rather than extend the caller's timeout.
        let now = Instant::now();
        let expires_at = request_expiry(
            request.deadline,
            Time::from_wallclock(SystemTime::now()),
            now,
        );
        Some(Pending {
            request,
            reply,
            received_at,
            trace,
            expires_at,
        })
    }
}

pub(super) enum InferenceReply {
    Walk(ServiceReply<WalkInferenceService>),
    Kick(ServiceReply<KickInferenceService>),
    GetUp(ServiceReply<GetUpInferenceService>),
}
impl InferenceReply {
    pub fn id(&self) -> &RequestId {
        match self {
            Self::Walk(reply) => reply.id(),
            Self::Kick(reply) => reply.id(),
            Self::GetUp(reply) => reply.id(),
        }
    }

    pub async fn respond(self, result: InferenceResult<Box<Joints<MotorCommand>>>) -> bool {
        let id = self.id().clone();
        let result = match self {
            Self::Walk(reply) => reply.reply_async(&result.map(lower_body)).await,
            Self::Kick(reply) => reply.reply_async(&result.map(lower_body)).await,
            Self::GetUp(reply) => reply.reply_async(&result.map(|boxed| *boxed)).await,
        };
        report_reply_result(id, result)
    }
}
fn report_reply_result(request_id: RequestId, result: ros_z::Result<()>) -> bool {
    if let Err(error) = result {
        warn!(?request_id, %error, "failed to send inference response");
        return false;
    }
    true
}

fn lower_body(output: Box<Joints<MotorCommand>>) -> LowerBodyJoints<MotorCommand> {
    LowerBodyJoints::from(BodyJoints::from(*output))
}
