use std::sync::Arc;

use alsa::{
    pcm::{Access, Format, HwParams},
    Direction, ValueOr, PCM,
};
use color_eyre::{eyre::WrapErr, Result};
use types::hardware::Samples;

pub struct Microphones {
    device: PCM,
}

impl Microphones {
    const SAMPLE_RATE: u32 = 44100;
    const NUMBER_OF_CHANNELS: usize = 4;
    const NUMBER_OF_SAMPLES: usize = 2048;

    pub fn new() -> Result<Self> {
        let device = PCM::new("default", Direction::Capture, false)
            .wrap_err("failed to open audio device")?;
        {
            let hardware_parameters =
                HwParams::any(&device).wrap_err("failed to create hardware parameters")?;
            hardware_parameters
                .set_access(Access::RWInterleaved)
                .wrap_err("failed to set access")?;
            hardware_parameters
                .set_format(Format::FloatLE)
                .wrap_err("failed to set format")?;
            hardware_parameters
                .set_rate_near(Self::SAMPLE_RATE, ValueOr::Nearest)
                .wrap_err("failed to set sample rate")?;
            hardware_parameters
                .set_channels(Self::NUMBER_OF_CHANNELS as u32)
                .wrap_err("failed to set channel")?;
            device
                .hw_params(&hardware_parameters)
                .wrap_err("failed to set hardware parameters")?;
        }
        device.prepare().wrap_err("failed to prepare device")?;
        Ok(Self { device })
    }

    pub fn read_from_microphones(&self) -> Result<Samples> {
        let io_device = self
            .device
            .io_f32()
            .wrap_err("failed to create I/O device")?;
        let mut interleaved_buffer = [0.0; Self::NUMBER_OF_CHANNELS * Self::NUMBER_OF_SAMPLES];
        let number_of_frames = io_device
            .readi(&mut interleaved_buffer)
            .wrap_err("failed to read audio data")?;
        let mut non_interleaved_buffer =
            vec![Vec::with_capacity(number_of_frames); Self::NUMBER_OF_CHANNELS];
        for (channel_index, non_interleaved_buffer) in non_interleaved_buffer.iter_mut().enumerate()
        {
            non_interleaved_buffer.extend(
                interleaved_buffer
                    .iter()
                    .skip(channel_index)
                    .step_by(Self::NUMBER_OF_CHANNELS),
            );
        }
        Ok(Samples {
            rate: Self::SAMPLE_RATE,
            channels_of_samples: Arc::new(non_interleaved_buffer),
        })
    }
}
