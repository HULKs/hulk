use std::time::SystemTime;

use booster::{
    ButtonEventMsg, FallDownState, LowCommand, LowState, RemoteControllerState, TFMessage,
};
use serde::{Deserialize, Serialize};
use zed::RGBDSensors;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SimulationMessage<T> {
    pub time: SystemTime,
    pub payload: T,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum ServerMessageKind {
    LowState(LowState),
    FallDownState(FallDownState),
    ButtonEventMsg(ButtonEventMsg),
    RemoteControllerState(RemoteControllerState),
    TFMessage(TFMessage),
    RGBDSensors(Box<RGBDSensors>),
}

#[derive(Debug, Serialize, Deserialize)]
pub enum ClientMessageKind {
    LowCommand(LowCommand),
}
