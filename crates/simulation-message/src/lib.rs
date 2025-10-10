use std::time::SystemTime;

use booster::{ButtonEventMsg, FallDownState, LowCommand, LowState, RemoteControllerState};
use ros2::geometry_msgs::transform_stamped::TransformStamped;
use serde::{Deserialize, Serialize};
use zed::RGBDSensors;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SimulationMessage<T> {
    pub time: SystemTime,
    pub payload: T,
}

impl<T> SimulationMessage<T> {
    pub fn new(time: SystemTime, payload: T) -> Self {
        Self { time, payload }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub enum ServerMessageKind {
    LowState(LowState),
    FallDownState(FallDownState),
    ButtonEventMsg(ButtonEventMsg),
    RemoteControllerState(RemoteControllerState),
    TransformStamped(TransformStamped),
    RGBDSensors(RGBDSensors),
}

#[derive(Debug, Serialize, Deserialize)]
pub enum ClientMessageKind {
    LowCommand(LowCommand),
}
