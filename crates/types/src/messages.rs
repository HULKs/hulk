use std::net::SocketAddr;

use serde::{Deserialize, Serialize};

use path_serde::{PathDeserialize, PathIntrospect, PathSerialize};
use spl_network_messages::{
    GameControllerReturnMessage, GameControllerStateMessage, GestureVisualRefereeMessage,
    HulkMessage,
};

#[derive(Clone, Debug, Deserialize, Serialize, PathSerialize, PathDeserialize, PathIntrospect)]
pub enum IncomingMessage {
    GameController(SocketAddr, GameControllerStateMessage),
    Spl(HulkMessage),
}

impl Default for IncomingMessage {
    fn default() -> Self {
        Self::Spl(Default::default())
    }
}

#[derive(Clone, Debug, Deserialize, Serialize, PathSerialize, PathDeserialize, PathIntrospect)]
pub enum OutgoingMessage {
    GameController(SocketAddr, GameControllerReturnMessage),
    Spl(HulkMessage),
    VisualReferee(SocketAddr, GestureVisualRefereeMessage),
}

impl Default for OutgoingMessage {
    fn default() -> Self {
        Self::Spl(Default::default())
    }
}
