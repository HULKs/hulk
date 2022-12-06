use std::sync::Arc;

use alsa::{
    pcm::{Access, Format, HwParams},
    Direction, ValueOr, PCM,
};
use color_eyre::{eyre::WrapErr, Result};
use serde::{de::Error, Deserialize, Deserializer};
use types::hardware::Samples;

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

fn deserialize_access<'de, D>(deserializer: D) -> Result<Access, D::Error>
where
    D: Deserializer<'de>,
{
    let value = String::deserialize(deserializer)?;
    Ok(match value.as_str() {
        "MMapInterleaved" => Access::MMapInterleaved,
        "MMapNonInterleaved" => Access::MMapNonInterleaved,
        "MMapComplex" => Access::MMapComplex,
        "RWInterleaved" => Access::RWInterleaved,
        "RWNonInterleaved" => Access::RWNonInterleaved,
        _ => {
            return Err(Error::unknown_variant(
                value.as_str(),
                &[
                    "MMapInterleaved",
                    "MMapNonInterleaved",
                    "MMapComplex",
                    "RWInterleaved",
                    "RWNonInterleaved",
                ],
            ))
        }
    })
}

fn deserialize_format<'de, D>(deserializer: D) -> Result<Format, D::Error>
where
    D: Deserializer<'de>,
{
    let value = String::deserialize(deserializer)?;
    Ok(match value.as_str() {
        "Unknown" => Format::Unknown,
        "S8" => Format::S8,
        "U8" => Format::U8,
        "S16LE" => Format::S16LE,
        "S16BE" => Format::S16BE,
        "U16LE" => Format::U16LE,
        "U16BE" => Format::U16BE,
        "S24LE" => Format::S24LE,
        "S24BE" => Format::S24BE,
        "U24LE" => Format::U24LE,
        "U24BE" => Format::U24BE,
        "S32LE" => Format::S32LE,
        "S32BE" => Format::S32BE,
        "U32LE" => Format::U32LE,
        "U32BE" => Format::U32BE,
        "FloatLE" => Format::FloatLE,
        "FloatBE" => Format::FloatBE,
        "Float64LE" => Format::Float64LE,
        "Float64BE" => Format::Float64BE,
        "IEC958SubframeLE" => Format::IEC958SubframeLE,
        "IEC958SubframeBE" => Format::IEC958SubframeBE,
        "MuLaw" => Format::MuLaw,
        "ALaw" => Format::ALaw,
        "ImaAdPCM" => Format::ImaAdPCM,
        "MPEG" => Format::MPEG,
        "GSM" => Format::GSM,
        "Special" => Format::Special,
        "S243LE" => Format::S243LE,
        "S243BE" => Format::S243BE,
        "U243LE" => Format::U243LE,
        "U243BE" => Format::U243BE,
        "S203LE" => Format::S203LE,
        "S203BE" => Format::S203BE,
        "U203LE" => Format::U203LE,
        "U203BE" => Format::U203BE,
        "S183LE" => Format::S183LE,
        "S183BE" => Format::S183BE,
        "U183LE" => Format::U183LE,
        "U183BE" => Format::U183BE,
        "G72324" => Format::G72324,
        "G723241B" => Format::G723241B,
        "G72340" => Format::G72340,
        "G723401B" => Format::G723401B,
        "DSDU8" => Format::DSDU8,
        "DSDU16LE" => Format::DSDU16LE,
        "DSDU32LE" => Format::DSDU32LE,
        "DSDU16BE" => Format::DSDU16BE,
        "DSDU32BE" => Format::DSDU32BE,
        _ => {
            return Err(Error::unknown_variant(
                value.as_str(),
                &[
                    "Unknown",
                    "S8",
                    "U8",
                    "S16LE",
                    "S16BE",
                    "U16LE",
                    "U16BE",
                    "S24LE",
                    "S24BE",
                    "U24LE",
                    "U24BE",
                    "S32LE",
                    "S32BE",
                    "U32LE",
                    "U32BE",
                    "FloatLE",
                    "FloatBE",
                    "Float64LE",
                    "Float64BE",
                    "IEC958SubframeLE",
                    "IEC958SubframeBE",
                    "MuLaw",
                    "ALaw",
                    "ImaAdPCM",
                    "MPEG",
                    "GSM",
                    "Special",
                    "S243LE",
                    "S243BE",
                    "U243LE",
                    "U243BE",
                    "S203LE",
                    "S203BE",
                    "U203LE",
                    "U203BE",
                    "S183LE",
                    "S183BE",
                    "U183LE",
                    "U183BE",
                    "G72324",
                    "G723241B",
                    "G72340",
                    "G723401B",
                    "DSDU8",
                    "DSDU16LE",
                    "DSDU32LE",
                    "DSDU16BE",
                    "DSDU32BE",
                ],
            ))
        }
    })
}
