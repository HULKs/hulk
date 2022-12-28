use spl_network_messages::{GameControllerReturnMessage, GameControllerStateMessage, SplMessage};

#[derive(Clone, Debug)]
pub enum IncomingMessage {
    GameController(GameControllerStateMessage),
    Spl(SplMessage),
}

impl Default for IncomingMessage {
    fn default() -> Self {
        IncomingMessage::GameController(Default::default())
    }
}

#[derive(Clone, Debug)]
pub enum OutgoingMessage {
    GameController(GameControllerReturnMessage),
    Spl(SplMessage),
}

impl Default for OutgoingMessage {
    fn default() -> Self {
        OutgoingMessage::GameController(Default::default())
    }
}
