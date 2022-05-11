use anyhow::Context;
use parking_lot::Mutex;

use crate::hardware::{HardwareInterface, NUMBER_OF_AUDIO_CHANNELS, NUMBER_OF_AUDIO_SAMPLES};

pub fn record_microphone<Hardware>(
    hardware_interface: &Hardware,
) -> anyhow::Result<&Mutex<[[f32; NUMBER_OF_AUDIO_SAMPLES]; NUMBER_OF_AUDIO_CHANNELS]>>
where
    Hardware: HardwareInterface,
{
    hardware_interface
        .produce_audio_data()
        .context("Failed to record from the microphone")?;
    Ok(hardware_interface.get_audio_buffer())
}
