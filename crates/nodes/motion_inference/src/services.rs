use crate::{
    inference::InferenceCommand,
    node::{
        GETUP_INFERENCE_SERVICE, GetUpInferenceService, InferenceResult, KICK_INFERENCE_SERVICE,
        KickInferenceService, WALK_INFERENCE_SERVICE, WalkInferenceService,
    },
};
use kinematics::joints::{
    Joints,
    body::{BodyJoints, LowerBodyJoints},
};
use ros_z::{
    prelude::*,
    service::{ServiceReply, ServiceServer},
};
use types::motor_command::MotorCommand;

pub(super) struct Requests {
    walk: ServiceServer<WalkInferenceService>,
    kick: ServiceServer<KickInferenceService>,
    get_up: ServiceServer<GetUpInferenceService>,
}
impl Requests {
    pub async fn new(node: &Node, qos: QosProfile) -> ros_z::Result<Self> {
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
        })
    }
    pub async fn receive(&mut self) -> ros_z::Result<(InferenceCommand, InferenceReply)> {
        tokio::select! {
            received = self.walk.take_request_async() => {
                let (request, reply) = received?.into_parts();
                Ok((InferenceCommand::Walk(request), InferenceReply::Walk(reply)))
            }
            received = self.kick.take_request_async() => {
                let (request, reply) = received?.into_parts();
                Ok((InferenceCommand::Kick(request), InferenceReply::Kick(reply)))
            }
            received = self.get_up.take_request_async() => {
                let (request, reply) = received?.into_parts();
                Ok((InferenceCommand::GetUp(request), InferenceReply::GetUp(reply)))
            }
        }
    }
}

pub(super) enum InferenceReply {
    Walk(ServiceReply<WalkInferenceService>),
    Kick(ServiceReply<KickInferenceService>),
    GetUp(ServiceReply<GetUpInferenceService>),
}
impl InferenceReply {
    pub async fn respond(self, result: InferenceResult<Box<Joints<MotorCommand>>>) {
        match self {
            Self::Walk(reply) => {
                let _ = reply.reply_async(&result.map(lower_body)).await;
            }
            Self::Kick(reply) => {
                let _ = reply.reply_async(&result.map(lower_body)).await;
            }
            Self::GetUp(reply) => {
                let _ = reply
                    .reply_async(&result.map(|boxed| boxed.as_ref().clone()))
                    .await;
            }
        }
    }
}
fn lower_body(output: Box<Joints<MotorCommand>>) -> LowerBodyJoints<MotorCommand> {
    LowerBodyJoints::from(BodyJoints::from(*output))
}
