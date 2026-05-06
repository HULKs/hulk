use ros_z::{Message, ServiceTypeInfo, msg::Service};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Default, Message)]
#[message(name = "custom_msgs::NavigateToRequest")]
pub struct NavigateToRequest {
    pub target_x: f64,
    pub target_y: f64,
    pub max_speed: f64,
}

impl ros_z::msg::WireMessage for NavigateToRequest {
    type Codec = ros_z::msg::SerdeCdrCodec<NavigateToRequest>;
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, Message)]
#[message(name = "custom_msgs::NavigateToResponse")]
pub struct NavigateToResponse {
    pub success: bool,
    pub estimated_duration: f64,
    pub message: String,
}

impl ros_z::msg::WireMessage for NavigateToResponse {
    type Codec = ros_z::msg::SerdeCdrCodec<NavigateToResponse>;
}

pub struct NavigateTo;

impl ServiceTypeInfo for NavigateTo {
    fn service_type_info() -> ros_z::entity::TypeInfo {
        ros_z::entity::TypeInfo::new("custom_msgs::NavigateTo", None)
    }
}

impl Service for NavigateTo {
    type Request = NavigateToRequest;
    type Response = NavigateToResponse;
}
