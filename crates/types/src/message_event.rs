use std::net::SocketAddr;

use serde::{Deserialize, Serialize};
use spl_network_messages::GameControllerReturnMessage;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum MessageEvent<'buffer> {
    GameControllerReturnMessageToBeSent {
        message: GameControllerReturnMessage,
    },
    SplMessageToBeSent {
        message: GameControllerReturnMessage,
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
