use std::time::Duration;

use alsa::pcm::{Access as AlsAccess, Format as AlsFormat};
use ros_z::Message;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize, Message)]
pub struct Parameters {
    pub sample_rate: u32,
    pub number_of_channels: usize,
    pub number_of_samples: usize,
    pub number_of_retries: usize,
    pub retry_sleep_duration: Duration,
    pub hardware_device_name: String,
    pub target_channels: usize,
    pub access: Access,
    pub format: Format,
}

#[derive(Clone, Debug, Serialize, Deserialize, Message)]
pub enum Access {
    MMapInterleaved,
    MMapNonInterleaved,
    MMapComplex,
    RWInterleaved,
    RWNonInterleaved,
}

impl From<&Access> for AlsAccess {
    fn from(a: &Access) -> Self {
        match *a {
            Access::MMapInterleaved => AlsAccess::MMapInterleaved,
            Access::MMapNonInterleaved => AlsAccess::MMapNonInterleaved,
            Access::MMapComplex => AlsAccess::MMapComplex,
            Access::RWInterleaved => AlsAccess::RWInterleaved,
            Access::RWNonInterleaved => AlsAccess::RWNonInterleaved,
        }
    }
}

impl From<&AlsAccess> for Access {
    fn from(a: &AlsAccess) -> Self {
        match *a {
            AlsAccess::MMapInterleaved => Access::MMapInterleaved,
            AlsAccess::MMapNonInterleaved => Access::MMapNonInterleaved,
            AlsAccess::MMapComplex => Access::MMapComplex,
            AlsAccess::RWInterleaved => Access::RWInterleaved,
            AlsAccess::RWNonInterleaved => Access::RWNonInterleaved,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, Message)]
pub enum Format {
    Unknown,
    S8,
    U8,
    S16LE,
    S16BE,
    S20LE,
    S20BE,
    U20LE,
    U20BE,
    U16LE,
    U16BE,
    S24LE,
    S24BE,
    U24LE,
    U24BE,
    S32LE,
    S32BE,
    U32LE,
    U32BE,
    FloatLE,
    FloatBE,
    Float64LE,
    Float64BE,
    IEC958SubframeLE,
    IEC958SubframeBE,
    MuLaw,
    ALaw,
    ImaAdPCM,
    MPEG,
    GSM,
    Special,
    S243LE,
    S243BE,
    U243LE,
    U243BE,
    S203LE,
    S203BE,
    U203LE,
    U203BE,
    S183LE,
    S183BE,
    U183LE,
    U183BE,
    G72324,
    G723241B,
    G72340,
    G723401B,
    DSDU8,
    DSDU16LE,
    DSDU32LE,
    DSDU16BE,
    DSDU32BE,
}

impl From<&Format> for AlsFormat {
    fn from(f: &Format) -> Self {
        match *f {
            Format::Unknown => AlsFormat::Unknown,
            Format::S8 => AlsFormat::S8,
            Format::U8 => AlsFormat::U8,
            Format::S16LE => AlsFormat::S16LE,
            Format::S16BE => AlsFormat::S16BE,
            Format::S20LE => AlsFormat::S20LE,
            Format::S20BE => AlsFormat::S20BE,
            Format::U20LE => AlsFormat::U20LE,
            Format::U20BE => AlsFormat::U20BE,
            Format::U16LE => AlsFormat::U16LE,
            Format::U16BE => AlsFormat::U16BE,
            Format::S24LE => AlsFormat::S24LE,
            Format::S24BE => AlsFormat::S24BE,
            Format::U24LE => AlsFormat::U24LE,
            Format::U24BE => AlsFormat::U24BE,
            Format::S32LE => AlsFormat::S32LE,
            Format::S32BE => AlsFormat::S32BE,
            Format::U32LE => AlsFormat::U32LE,
            Format::U32BE => AlsFormat::U32BE,
            Format::FloatLE => AlsFormat::FloatLE,
            Format::FloatBE => AlsFormat::FloatBE,
            Format::Float64LE => AlsFormat::Float64LE,
            Format::Float64BE => AlsFormat::Float64BE,
            Format::IEC958SubframeLE => AlsFormat::IEC958SubframeLE,
            Format::IEC958SubframeBE => AlsFormat::IEC958SubframeBE,
            Format::MuLaw => AlsFormat::MuLaw,
            Format::ALaw => AlsFormat::ALaw,
            Format::ImaAdPCM => AlsFormat::ImaAdPCM,
            Format::MPEG => AlsFormat::MPEG,
            Format::GSM => AlsFormat::GSM,
            Format::Special => AlsFormat::Special,
            Format::S243LE => AlsFormat::S243LE,
            Format::S243BE => AlsFormat::S243BE,
            Format::U243LE => AlsFormat::U243LE,
            Format::U243BE => AlsFormat::U243BE,
            Format::S203LE => AlsFormat::S203LE,
            Format::S203BE => AlsFormat::S203BE,
            Format::U203LE => AlsFormat::U203LE,
            Format::U203BE => AlsFormat::U203BE,
            Format::S183LE => AlsFormat::S183LE,
            Format::S183BE => AlsFormat::S183BE,
            Format::U183LE => AlsFormat::U183LE,
            Format::U183BE => AlsFormat::U183BE,
            Format::G72324 => AlsFormat::G72324,
            Format::G723241B => AlsFormat::G723241B,
            Format::G72340 => AlsFormat::G72340,
            Format::G723401B => AlsFormat::G723401B,
            Format::DSDU8 => AlsFormat::DSDU8,
            Format::DSDU16LE => AlsFormat::DSDU16LE,
            Format::DSDU32LE => AlsFormat::DSDU32LE,
            Format::DSDU16BE => AlsFormat::DSDU16BE,
            Format::DSDU32BE => AlsFormat::DSDU32BE,
        }
    }
}

impl From<&AlsFormat> for Format {
    fn from(f: &AlsFormat) -> Self {
        match *f {
            AlsFormat::Unknown => Format::Unknown,
            AlsFormat::S8 => Format::S8,
            AlsFormat::U8 => Format::U8,
            AlsFormat::S16LE => Format::S16LE,
            AlsFormat::S16BE => Format::S16BE,
            AlsFormat::S20LE => Format::S20LE,
            AlsFormat::S20BE => Format::S20BE,
            AlsFormat::U20LE => Format::U20LE,
            AlsFormat::U20BE => Format::U20BE,
            AlsFormat::U16LE => Format::U16LE,
            AlsFormat::U16BE => Format::U16BE,
            AlsFormat::S24LE => Format::S24LE,
            AlsFormat::S24BE => Format::S24BE,
            AlsFormat::U24LE => Format::U24LE,
            AlsFormat::U24BE => Format::U24BE,
            AlsFormat::S32LE => Format::S32LE,
            AlsFormat::S32BE => Format::S32BE,
            AlsFormat::U32LE => Format::U32LE,
            AlsFormat::U32BE => Format::U32BE,
            AlsFormat::FloatLE => Format::FloatLE,
            AlsFormat::FloatBE => Format::FloatBE,
            AlsFormat::Float64LE => Format::Float64LE,
            AlsFormat::Float64BE => Format::Float64BE,
            AlsFormat::IEC958SubframeLE => Format::IEC958SubframeLE,
            AlsFormat::IEC958SubframeBE => Format::IEC958SubframeBE,
            AlsFormat::MuLaw => Format::MuLaw,
            AlsFormat::ALaw => Format::ALaw,
            AlsFormat::ImaAdPCM => Format::ImaAdPCM,
            AlsFormat::MPEG => Format::MPEG,
            AlsFormat::GSM => Format::GSM,
            AlsFormat::Special => Format::Special,
            AlsFormat::S243LE => Format::S243LE,
            AlsFormat::S243BE => Format::S243BE,
            AlsFormat::U243LE => Format::U243LE,
            AlsFormat::U243BE => Format::U243BE,
            AlsFormat::S203LE => Format::S203LE,
            AlsFormat::S203BE => Format::S203BE,
            AlsFormat::U203LE => Format::U203LE,
            AlsFormat::U203BE => Format::U203BE,
            AlsFormat::S183LE => Format::S183LE,
            AlsFormat::S183BE => Format::S183BE,
            AlsFormat::U183LE => Format::U183LE,
            AlsFormat::U183BE => Format::U183BE,
            AlsFormat::G72324 => Format::G72324,
            AlsFormat::G723241B => Format::G723241B,
            AlsFormat::G72340 => Format::G72340,
            AlsFormat::G723401B => Format::G723401B,
            AlsFormat::DSDU8 => Format::DSDU8,
            AlsFormat::DSDU16LE => Format::DSDU16LE,
            AlsFormat::DSDU32LE => Format::DSDU32LE,
            AlsFormat::DSDU16BE => Format::DSDU16BE,
            AlsFormat::DSDU32BE => Format::DSDU32BE,
            _ => Format::Unknown,
        }
    }
}
