use ros_z::{Message, ServiceTypeInfo, entity::TypeInfo, message::Service};
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
    fn service_type_info() -> TypeInfo {
        let descriptor = ros_z_schema::ServiceDef::new(
            "custom_msgs::NavigateTo",
            NavigateToRequest::type_name(),
            NavigateToResponse::type_name(),
        )
        .expect("navigation service descriptor should be static and valid");
        let hash = ros_z_schema::compute_hash(&descriptor)
            .expect("navigation service hash should be static and valid");
        TypeInfo::new(descriptor.type_name.as_str(), hash)
    }
}

impl Service for NavigateTo {
    type Request = NavigateToRequest;
    type Response = NavigateToResponse;
}
