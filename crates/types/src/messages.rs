use spl_network_messages::{GameControllerReturnMessage, GameControllerStateMessage, SplMessage};

#[derive(Clone, Debug)]
pub enum IncomingMessage {
    GameController(GameControllerStateMessage),
    Spl(SplMessage),
}

#[derive(Clone, Debug)]
pub enum OutgoingMessage {
    GameController(GameControllerReturnMessage),
    Spl(SplMessage),
}
