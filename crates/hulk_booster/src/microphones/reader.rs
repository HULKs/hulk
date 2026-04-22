use std::{sync::Arc, thread::sleep};

use alsa::{Direction, PCM, ValueOr, pcm::HwParams};
use color_eyre::{
    Result,
    eyre::{WrapErr, eyre},
};
use log::warn;
use types::samples::Samples;

use crate::microphones::Parameters;

pub struct Microphones {
    device: PCM,
    parameters: Parameters,
}

impl Microphones {
    pub fn new(parameters: Parameters) -> Result<Self> {
        let device = create_device(&parameters).wrap_err("failed to create device")?;
        Ok(Self { device, parameters })
    }

    pub fn retrying_read(&mut self) -> Result<Samples> {
        let number_of_retries = self.parameters.number_of_retries;
        for _ in 0..number_of_retries {
            match self.read() {
                Ok(samples) => return Ok(samples),
                Err(error) => {
                    warn!("failed to read from microphones: {error:#?}");
                    sleep(self.parameters.retry_sleep_duration);
                    self.device = match create_device(&self.parameters) {
                        Ok(device) => device,
                        Err(error) => {
                            warn!("failed to create device: {error:#?}");
                            continue;
                        }
                    };
                }
            }
        }
        Err(eyre!(
            "failed to read from microphones after {number_of_retries}, giving up..."
        ))
    }

    fn read(&self) -> std::result::Result<Samples, color_eyre::eyre::Error> {
        let io_device = self
            .device
            .io_i16()
            .wrap_err("failed to create I/O device")?;

        let mut interleaved_buffer =
            vec![0i16; self.parameters.number_of_channels * self.parameters.number_of_samples];

        let number_of_frames = io_device
            .readi(&mut interleaved_buffer)
            .wrap_err("failed to read audio data")?;

        // only want the first 3 channels
        let mut non_interleaved_buffer =
            vec![Vec::with_capacity(number_of_frames); self.parameters.target_channels];

        for (channel_index, non_interleaved_buffer) in non_interleaved_buffer.iter_mut().enumerate()
        {
            non_interleaved_buffer.extend(
                interleaved_buffer
                    .iter()
                    .skip(channel_index)
                    .step_by(self.parameters.number_of_channels)
                    .map(|&sample| sample as f32 / i16::MAX as f32),
            );
        }

        Ok(Samples {
            rate: self.parameters.sample_rate,
            channels_of_samples: Arc::new(non_interleaved_buffer),
        })
    }
}

fn create_device(parameters: &Parameters) -> Result<PCM> {
    let device = PCM::new(
        parameters.hardware_device_name.as_str(),
        Direction::Capture,
        false,
    )
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
    Ok(device)
}
