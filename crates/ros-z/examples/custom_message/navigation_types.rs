use ros_z::{
    Message, ServiceTypeInfo,
    entity::{SchemaHash, TypeInfo},
    message::Service,
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Default, Message)]
#[message(name = "custom_msgs::NavigateToRequest")]
pub struct NavigateToRequest {
    pub target_x: f64,
    pub target_y: f64,
    pub max_speed: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, Message)]
#[message(name = "custom_msgs::NavigateToResponse")]
pub struct NavigateToResponse {
    pub success: bool,
    pub estimated_duration: f64,
    pub message: String,
}

pub struct NavigateTo;

impl ServiceTypeInfo for NavigateTo {
    fn service_type_info() -> Result<TypeInfo, ros_z_schema::SchemaError> {
        let descriptor = ros_z_schema::ServiceDef::new(
            "custom_msgs::NavigateTo",
            "custom_msgs::NavigateToRequest",
            "custom_msgs::NavigateToResponse",
        )?;
        Ok(TypeInfo::new(
            "custom_msgs::NavigateTo",
            Some(SchemaHash(ros_z_schema::compute_hash(&descriptor).0)),
        ))
    }
}

impl Service for NavigateTo {
    type Request = NavigateToRequest;
    type Response = NavigateToResponse;
}
