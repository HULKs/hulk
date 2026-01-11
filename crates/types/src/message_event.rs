use std::net::SocketAddr;

use hsl_network_messages::{GameControllerReturnMessage, HulkMessage};
use serde::{Deserialize, Serialize};

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
