use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, ros_z::Message)]
pub struct ControllerInput {
    pub connected: bool,
    pub device_name: String,
    pub axes: Vec<ControllerAxis>,
    pub buttons: Vec<ControllerButton>,
}

impl ControllerInput {
    pub fn axis_value(&self, name: Axis) -> f32 {
        self.axes
            .iter()
            .find(|axis| axis.name == name)
            .map_or(0.0, |axis| axis.value)
    }

    pub fn button_value(&self, name: Button) -> f32 {
        self.buttons
            .iter()
            .find(|button| button.name == name)
            .map_or(0.0, |button| button.value)
    }

    pub fn is_pressed(&self, name: Button) -> bool {
        self.buttons
            .iter()
            .any(|button| button.name == name && button.pressed)
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ros_z::Message)]
pub struct ControllerAxis {
    pub name: Axis,
    pub value: f32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ros_z::Message)]
pub struct ControllerButton {
    pub name: Button,
    pub pressed: bool,
    pub value: f32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ros_z::Message)]
pub enum Axis {
    LeftStickX,
    LeftStickY,
    LeftZ,
    RightStickX,
    RightStickY,
    RightZ,
    DPadX,
    DPadY,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ros_z::Message)]
pub enum Button {
    South,
    East,
    North,
    West,
    C,
    Z,
    LeftTrigger,
    LeftTrigger2,
    RightTrigger,
    RightTrigger2,
    Select,
    Start,
    Mode,
    LeftThumb,
    RightThumb,
    DPadUp,
    DPadDown,
    DPadLeft,
    DPadRight,
    Unknown,
}
