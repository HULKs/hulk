use color_eyre::Result;
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

    pub fn cycle(&mut self, _context: CycleContext<impl Interface>) -> Result<MainOutputs> {
        Ok(MainOutputs::default())
        //hardware_interface
        //    .produce_audio_data()
        //    .context("Failed to record from the microphone")?;
        //Ok(hardware_interface.get_audio_buffer())
    }
}
