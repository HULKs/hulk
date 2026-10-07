use std::net::SocketAddr;

use ros_z::Message;
use serde::{Deserialize, Serialize};

use hsl_network_messages::{GameControllerReturnMessage, GameControllerStateMessage, HulkMessage};

#[derive(Clone, Debug, Deserialize, Serialize, Message)]
pub enum IncomingMessage {
    GameController(SocketAddr, GameControllerStateMessage),
    Hsl(HulkMessage),
}

#[derive(Clone, Debug, Deserialize, Serialize, Message)]
pub enum OutgoingMessage {
    GameController(SocketAddr, GameControllerReturnMessage),
    Hsl(HulkMessage),
}
