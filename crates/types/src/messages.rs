use std::net::SocketAddr;

use ros_z::Message;
use serde::{Deserialize, Serialize};

use hsl_network_messages::{GameControllerReturnMessage, GameControllerStateMessage, HulkMessage};
use path_serde::{PathDeserialize, PathIntrospect, PathSerialize};

#[derive(
    Clone, Debug, Deserialize, Serialize, PathSerialize, PathDeserialize, PathIntrospect, Message,
)]
pub enum IncomingMessage {
    GameController(SocketAddr, GameControllerStateMessage),
    Hsl(HulkMessage),
}

impl Default for IncomingMessage {
    fn default() -> Self {
        Self::Hsl(Default::default())
    }
}

#[derive(
    Clone, Debug, Deserialize, Serialize, PathSerialize, PathDeserialize, PathIntrospect, Message,
)]
pub enum OutgoingMessage {
    GameController(SocketAddr, GameControllerReturnMessage),
    Hsl(HulkMessage),
}

impl Default for OutgoingMessage {
    fn default() -> Self {
        Self::Hsl(Default::default())
    }
}
