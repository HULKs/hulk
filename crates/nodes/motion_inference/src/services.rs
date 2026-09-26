use crate::{
    inference::{InferenceCommand, InferenceOutput, InferenceRequest, InferenceResponse},
    node::{
        GETUP_INFERENCE_SERVICE, GetUpInferenceService, InferenceResult, KICK_INFERENCE_SERVICE,
        KickInferenceService, WALK_INFERENCE_SERVICE, WalkInferenceService,
    },
};
use kinematics::joints::body::{BodyJoints, LowerBodyJoints};
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
    pub async fn receive(
        &mut self,
    ) -> ros_z::Result<(InferenceRequest<InferenceCommand>, InferenceReply)> {
        tokio::select! {
            received = self.walk.take_request_async() => {
                let (request, reply) = received?.into_parts();
                Ok((request.map_command(InferenceCommand::Walk), InferenceReply::Walk(reply)))
            }
            received = self.kick.take_request_async() => {
                let (request, reply) = received?.into_parts();
                Ok((request.map_command(InferenceCommand::Kick), InferenceReply::Kick(reply)))
            }
            received = self.get_up.take_request_async() => {
                let (request, reply) = received?.into_parts();
                Ok((request.map_command(InferenceCommand::GetUp), InferenceReply::GetUp(reply)))
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
    pub async fn respond(self, result: InferenceResult<InferenceOutput>) {
        match self {
            Self::Walk(reply) => {
                let _ = reply.reply_async(&result.map(lower_body)).await;
            }
            Self::Kick(reply) => {
                let _ = reply.reply_async(&result.map(lower_body)).await;
            }
            Self::GetUp(reply) => {
                let _ = reply
                    .reply_async(&result.map(|output| InferenceResponse {
                        joints: output.joints,
                        execution: output.execution,
                    }))
                    .await;
            }
        }
    }
}
fn lower_body(output: InferenceOutput) -> InferenceResponse<LowerBodyJoints<MotorCommand>> {
    InferenceResponse {
        joints: Box::new(LowerBodyJoints::from(BodyJoints::from(*output.joints))),
        execution: output.execution,
    }
}
