use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, ros_z::Message)]
pub struct ControllerInput {
    pub connected: bool,
    pub device_name: String,
    pub axes: Vec<ControllerAxis>,
    pub buttons: Vec<ControllerButton>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ros_z::Message)]
pub struct ControllerAxis {
    pub name: String,
    pub value: f32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ros_z::Message)]
pub struct ControllerButton {
    pub name: String,
    pub pressed: bool,
    pub value: f32,
}
