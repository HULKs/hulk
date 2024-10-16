use color_eyre::{eyre::Ok, Result};
use context_attribute::context;
use framework::MainOutput;
use serde::{Deserialize, Serialize};
use spl_network_messages::{HulkMessage, StrikerMessage, VisualRefereeMessage};
use types::messages::IncomingMessage;

#[derive(Deserialize, Serialize)]
pub struct MessageFilter {}

#[context]
pub struct CreationContext {}

#[context]
pub struct CycleContext {
    message: Input<IncomingMessage, "message">,
    jersey_number: Parameter<usize, "jersey_number">,
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
            IncomingMessage::GameController(source_address, message) => Some(
                IncomingMessage::GameController(*source_address, message.clone()),
            ),
            IncomingMessage::Spl(
                message @ (HulkMessage::Striker(StrikerMessage { jersey_number, .. })
                | HulkMessage::VisualReferee(VisualRefereeMessage {
                    jersey_number, ..
                })),
            ) if jersey_number != context.jersey_number => Some(IncomingMessage::Spl(*message)),
            _ => None,
        };
        Ok(MainOutputs {
            filtered_message: message.into(),
        })
    }
}
