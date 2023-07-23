use std::sync::Arc;

use alsa::{
    pcm::{Access, Format, HwParams},
    Direction, ValueOr, PCM,
};
use color_eyre::{eyre::WrapErr, Result};
use serde::Deserialize;
use types::samples::Samples;

use crate::audio_parameter_deserializers::{deserialize_access, deserialize_format};

pub struct Microphones {
    device: PCM,
    parameters: Parameters,
}

impl Microphones {
    pub fn new(parameters: Parameters) -> Result<Self> {
        let device = PCM::new("default", Direction::Capture, false)
            .wrap_err("failed to open audio device")?;
        {
            let hardware_parameters =
                HwParams::any(&device).wrap_err("failed to create hardware parameters")?;
            hardware_parameters
                .set_access(parameters.access)
                .wrap_err("failed to set access")?;
            hardware_parameters
                .set_format(parameters.format)
                .wrap_err("failed to set format")?;
            hardware_parameters
                .set_rate_near(parameters.sample_rate, ValueOr::Nearest)
                .wrap_err("failed to set sample rate")?;
            hardware_parameters
                .set_channels(parameters.number_of_channels as u32)
                .wrap_err("failed to set channel")?;
            device
                .hw_params(&hardware_parameters)
                .wrap_err("failed to set hardware parameters")?;
        }
        device.prepare().wrap_err("failed to prepare device")?;
        Ok(Self { device, parameters })
    }

    pub fn read_from_microphones(&self) -> Result<Samples> {
        let io_device = self
            .device
            .io_f32()
            .wrap_err("failed to create I/O device")?;
        let mut interleaved_buffer =
            vec![0.0; self.parameters.number_of_channels * self.parameters.number_of_samples];
        let number_of_frames = io_device
            .readi(&mut interleaved_buffer)
            .wrap_err("failed to read audio data")?;
        let mut non_interleaved_buffer =
            vec![Vec::with_capacity(number_of_frames); self.parameters.number_of_channels];
        for (channel_index, non_interleaved_buffer) in non_interleaved_buffer.iter_mut().enumerate()
        {
            non_interleaved_buffer.extend(
                interleaved_buffer
                    .iter()
                    .skip(channel_index)
                    .step_by(self.parameters.number_of_channels),
            );
        }
        Ok(Samples {
            rate: self.parameters.sample_rate,
            channels_of_samples: Arc::new(non_interleaved_buffer),
        })
    }
}

#[derive(Clone, Debug, Deserialize)]
pub struct Parameters {
    sample_rate: u32,
    number_of_channels: usize,
    number_of_samples: usize,

    #[serde(deserialize_with = "deserialize_access")]
    access: Access,
    #[serde(deserialize_with = "deserialize_format")]
    format: Format,
}
