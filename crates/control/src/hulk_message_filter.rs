use color_eyre::Result;
use context_attribute::context;
use framework::{MainOutput, PerceptionInput};
use spl_network_messages::HulkMessage;
use types::messages::IncomingMessage;

pub struct HulkMessageFilter {}

#[context]
pub struct CreationContext {}

#[context]
pub struct CycleContext {
    pub network_message: PerceptionInput<IncomingMessage, "SplNetwork", "message">,
}

#[context]
#[derive(Default)]
pub struct MainOutputs {
    pub hulk_messages: MainOutput<Vec<HulkMessage>>,
}

impl HulkMessageFilter {
    pub fn new(_context: CreationContext) -> Result<Self> {
        Ok(Self {})
    }

    pub fn cycle(&mut self, context: CycleContext) -> Result<MainOutputs> {
        let hulk_messages: Vec<_> = context
            .network_message
            .persistent
            .values()
            .flatten()
            .filter_map(|message| match message {
                IncomingMessage::GameController(_) => None,
                IncomingMessage::Spl(message) => Some(message),
            })
            .copied()
            .collect();

        Ok(MainOutputs {
            hulk_messages: hulk_messages.into(),
        })
    }
}
