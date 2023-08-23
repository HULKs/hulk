use color_eyre::{eyre::WrapErr, Result};
use context_attribute::context;
use framework::MainOutput;
use hardware::MicrophoneInterface;
use types::samples::Samples;

pub struct MicrophoneRecorder {}

#[context]
pub struct CreationContext {}

#[context]
pub struct CycleContext {
    hardware_interface: HardwareInterface,
}

#[context]
#[derive(Default)]
pub struct MainOutputs {
    pub samples: MainOutput<Samples>,
}

impl MicrophoneRecorder {
    pub fn new(_context: CreationContext) -> Result<Self> {
        Ok(Self {})
    }

    pub fn cycle(
        &mut self,
        context: CycleContext<impl MicrophoneInterface>,
    ) -> Result<MainOutputs> {
        let samples = context
            .hardware_interface
            .read_from_microphones()
            .wrap_err("failed to read from microphones")?;
        Ok(MainOutputs {
            samples: samples.into(),
        })
    }
}
