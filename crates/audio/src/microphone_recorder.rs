use color_eyre::{eyre::WrapErr, Result};
use context_attribute::context;
use framework::MainOutput;
use types::hardware::Interface;

pub struct MicrophoneRecorder {}

#[context]
pub struct CreationContext {}

#[context]
pub struct CycleContext {
    pub hardware_interface: HardwareInterface,
}

#[context]
#[derive(Default)]
pub struct MainOutputs {
    pub buffer: MainOutput<bool>,
}

impl MicrophoneRecorder {
    pub fn new(_context: CreationContext) -> Result<Self> {
        Ok(Self {})
    }

    pub fn cycle(&mut self, context: CycleContext<impl Interface>) -> Result<MainOutputs> {
        let _samples = context
            .hardware_interface
            .read_from_microphones()
            .wrap_err("failed to read from microphones")?;
        Ok(MainOutputs::default())
    }
}
