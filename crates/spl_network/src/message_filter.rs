use color_eyre::{eyre::Ok, Result};
use context_attribute::context;
use framework::MainOutput;
use serde::{Deserialize, Serialize};
use spl_network_messages::PlayerNumber;
use types::messages::IncomingMessage;

#[derive(Deserialize, Serialize)]
pub struct MessageFilter {}

#[context]
pub struct CreationContext {}

#[context]
pub struct CycleContext {
    message: Input<IncomingMessage, "message">,
    player_number: Parameter<PlayerNumber, "player_number">,
}

#[context]
pub struct MainOutputs {
    pub filtered_message: MainOutput<Option<IncomingMessage>>,
}

impl MessageFilter {
    pub fn new(_context: CreationContext) -> Result<Self> {
        Ok(Self {})
    }

    pub fn cycle(&mut self, context: CycleContext) -> Result<MainOutputs> {
        let message = match context.message {
            IncomingMessage::GameController(message) => {
                Some(IncomingMessage::GameController(message.clone()))
            }
            IncomingMessage::Spl(message) if message.player_number == *context.player_number => {
                Some(IncomingMessage::Spl(*message))
            }
            _ => None,
        };
        Ok(MainOutputs {
            filtered_message: message.into(),
        })
    }
}
