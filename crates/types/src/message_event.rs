use std::net::SocketAddr;

use serde::{Deserialize, Serialize};
use hsl_network_messages::{GameControllerReturnMessage, HulkMessage};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum MessageEvent<'buffer> {
    GameControllerReturnMessageToBeSent {
        message: GameControllerReturnMessage,
    },
    HslMessageToBeSent {
        message: HulkMessage,
    },
    IncomingGameControllerStateMessage {
        message: &'buffer [u8],
        sender: SocketAddr,
    },
    IncomingHslMessage {
        message: &'buffer [u8],
        sender: SocketAddr,
    },
}
