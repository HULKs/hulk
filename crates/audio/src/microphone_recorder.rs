use std::{thread::sleep, time::Duration};

use color_eyre::{eyre::WrapErr, Result};
use context_attribute::context;
use framework::MainOutput;
use types::hardware::Interface;

pub struct MicrophoneRecorder {}

#[context]
pub struct NewContext {}

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
    pub fn new(_context: NewContext) -> Result<Self> {
        Ok(Self {})
    }

    pub fn cycle(&mut self, context: CycleContext<impl Interface>) -> Result<MainOutputs> {
        let samples = context
            .hardware_interface
            .read_from_microphones()
            .wrap_err("failed to read from microphones")?;
        sleep(Duration::from_secs(1));
        Ok(MainOutputs::default())
        //hardware_interface
        //    .produce_audio_data()
        //    .context("Failed to record from the microphone")?;
        //Ok(hardware_interface.get_audio_buffer())
    }
}
