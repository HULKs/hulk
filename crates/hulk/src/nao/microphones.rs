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
    let value = <&'de str>::deserialize(deserializer)?;
    match value {
        "MMapInterleaved" => Ok(Access::MMapInterleaved),
        "MMapNonInterleaved" => Ok(Access::MMapNonInterleaved),
        "MMapComplex" => Ok(Access::MMapComplex),
        "RWInterleaved" => Ok(Access::RWInterleaved),
        "RWNonInterleaved" => Ok(Access::RWNonInterleaved),
        _ => Err(Error::unknown_variant(
            value,
            &[
                "MMapInterleaved",
                "MMapNonInterleaved",
                "MMapComplex",
                "RWInterleaved",
                "RWNonInterleaved",
            ],
        )),
    }
}

fn deserialize_format<'de, D>(deserializer: D) -> Result<Format, D::Error>
where
    D: Deserializer<'de>,
{
    let value = <&'de str>::deserialize(deserializer)?;
    match value {
        "Unknown" => Ok(Format::Unknown),
        "S8" => Ok(Format::S8),
        "U8" => Ok(Format::U8),
        "S16LE" => Ok(Format::S16LE),
        "S16BE" => Ok(Format::S16BE),
        "U16LE" => Ok(Format::U16LE),
        "U16BE" => Ok(Format::U16BE),
        "S24LE" => Ok(Format::S24LE),
        "S24BE" => Ok(Format::S24BE),
        "U24LE" => Ok(Format::U24LE),
        "U24BE" => Ok(Format::U24BE),
        "S32LE" => Ok(Format::S32LE),
        "S32BE" => Ok(Format::S32BE),
        "U32LE" => Ok(Format::U32LE),
        "U32BE" => Ok(Format::U32BE),
        "FloatLE" => Ok(Format::FloatLE),
        "FloatBE" => Ok(Format::FloatBE),
        "Float64LE" => Ok(Format::Float64LE),
        "Float64BE" => Ok(Format::Float64BE),
        "IEC958SubframeLE" => Ok(Format::IEC958SubframeLE),
        "IEC958SubframeBE" => Ok(Format::IEC958SubframeBE),
        "MuLaw" => Ok(Format::MuLaw),
        "ALaw" => Ok(Format::ALaw),
        "ImaAdPCM" => Ok(Format::ImaAdPCM),
        "MPEG" => Ok(Format::MPEG),
        "GSM" => Ok(Format::GSM),
        "Special" => Ok(Format::Special),
        "S243LE" => Ok(Format::S243LE),
        "S243BE" => Ok(Format::S243BE),
        "U243LE" => Ok(Format::U243LE),
        "U243BE" => Ok(Format::U243BE),
        "S203LE" => Ok(Format::S203LE),
        "S203BE" => Ok(Format::S203BE),
        "U203LE" => Ok(Format::U203LE),
        "U203BE" => Ok(Format::U203BE),
        "S183LE" => Ok(Format::S183LE),
        "S183BE" => Ok(Format::S183BE),
        "U183LE" => Ok(Format::U183LE),
        "U183BE" => Ok(Format::U183BE),
        "G72324" => Ok(Format::G72324),
        "G723241B" => Ok(Format::G723241B),
        "G72340" => Ok(Format::G72340),
        "G723401B" => Ok(Format::G723401B),
        "DSDU8" => Ok(Format::DSDU8),
        "DSDU16LE" => Ok(Format::DSDU16LE),
        "DSDU32LE" => Ok(Format::DSDU32LE),
        "DSDU16BE" => Ok(Format::DSDU16BE),
        "DSDU32BE" => Ok(Format::DSDU32BE),
        _ => Err(Error::unknown_variant(
            value,
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
        )),
    }
}
