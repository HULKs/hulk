use kinematics::joints::Joints;
use serde::{Deserialize, Serialize};

use coordinate_systems::{Ground, Walk};
use linear_algebra::{Point2, Vector2};
use ros_z::{prelude::*, time::Time};
use types::robot_command::MotorCommand;

#[derive(Clone, Serialize, Deserialize, Message)]
pub enum InferenceRequest {
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

#[derive(Message, Clone, Serialize, Deserialize)]
pub struct KickRequest {
    pub ball_position: Point2<Ground>,
    pub ball_velocity: Vector2<Ground>,
    pub direction: f32,
    pub target_speed: f32,
    pub strong: bool,
    pub quick: bool,
}

#[derive(Clone, Serialize, Deserialize, Message)]
pub enum InferenceResult {
    Output(Output),
    RequestDenied { reason: String },
}

pub struct InferenceService;

impl Service for InferenceService {
    type Request = InferenceRequest;
    type Response = InferenceResult;
}

#[derive(Clone, Serialize, Deserialize, Message)]
pub struct Output {
    pub inference: InferenceOutput,
    pub valid_until: Time,
}

#[derive(Clone, Serialize, Deserialize, Message)]
pub struct InferenceOutput {
    pub joints: Box<Joints<MotorCommand>>,
    pub mode: Mode,
}

#[derive(Clone, Default, PartialEq, Eq, Serialize, Deserialize, Message)]
pub enum Mode {
    Full,
    #[default]
    Body,
}

impl ServiceTypeInfo for InferenceService {
    fn service_type_info() -> TypeInfo {
        let descriptor = ros_z_schema::ServiceDef::new(
            "motion_inference::node::InferenceService",
            InferenceRequest::type_name(),
            InferenceResult::type_name(),
        )
        .expect("static inference service descriptor is valid");
        let hash = ros_z_schema::compute_hash(&descriptor).expect("static service hash is valid");
        TypeInfo::new(descriptor.type_name.as_str(), hash)
    }
}
