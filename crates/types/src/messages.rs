use std::net::SocketAddr;

use serde::{Deserialize, Serialize};

use serialize_hierarchy::SerializeHierarchy;
use spl_network_messages::{
    GameControllerReturnMessage, GameControllerStateMessage, HulkMessage, VisualRefereeMessage,
};

#[derive(Clone, Debug, Deserialize, Serialize, SerializeHierarchy)]
pub enum IncomingMessage {
    GameController(SocketAddr, GameControllerStateMessage),
    Spl(HulkMessage),
}

impl Default for IncomingMessage {
    fn default() -> Self {
        Self::Spl(Default::default())
    }
}

#[derive(Clone, Debug, Deserialize, Serialize, SerializeHierarchy)]
pub enum OutgoingMessage {
    GameController(SocketAddr, GameControllerReturnMessage),
    Spl(HulkMessage),
    VisualReferee(SocketAddr, VisualRefereeMessage),
}

impl Default for OutgoingMessage {
    fn default() -> Self {
        Self::Spl(Default::default())
    }
}
