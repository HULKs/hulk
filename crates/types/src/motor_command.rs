use ros_z::Message;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Message, Clone)]
pub struct MotorCommand {
    pub position: f32,
    pub velocity: f32,
    pub torque: f32,
    pub kp: f32,
    pub kd: f32,
}

impl MotorCommand {
    pub fn zeros() -> Self {
        Self {
            position: 0.0,
            velocity: 0.0,
            torque: 0.0,
            kp: 0.0,
            kd: 0.0,
        }
    }
}
