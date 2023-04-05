use std::net::SocketAddr;

use serde::{Deserialize, Serialize};
use spl_network_messages::{GameControllerReturnMessage, HulkMessage};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum MessageEvent<'buffer> {
    GameControllerReturnMessageToBeSent {
        message: GameControllerReturnMessage,
    },
    SplMessageToBeSent {
        message: HulkMessage,
    },
    IncomingGameControllerStateMessage {
        message: &'buffer [u8],
        sender: SocketAddr,
    },
    IncomingSplMessage {
        message: &'buffer [u8],
        sender: SocketAddr,
    },
}
