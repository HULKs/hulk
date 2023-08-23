use color_eyre::{eyre::WrapErr, Result};
use context_attribute::context;
use framework::MainOutput;
use hardware::NetworkInterface;
use types::messages::IncomingMessage;

pub struct MessageReceiver {}

#[context]
pub struct CreationContext {}

#[context]
pub struct CycleContext {
    hardware_interface: HardwareInterface,
}

#[context]
pub struct MainOutputs {
    pub message: MainOutput<IncomingMessage>,
}

impl MessageReceiver {
    pub fn new(_context: CreationContext) -> Result<Self> {
        Ok(Self {})
    }

    pub fn cycle(&mut self, context: CycleContext<impl NetworkInterface>) -> Result<MainOutputs> {
        let message = context
            .hardware_interface
            .read_from_network()
            .wrap_err("failed to read from network")?;
        Ok(MainOutputs {
            message: message.into(),
        })
    }
}
