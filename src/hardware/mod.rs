mod interface;
#[cfg(feature = "nao")]
mod nao;
#[cfg(feature = "webots")]
mod webots;

pub use interface::{
    HardwareIds, HardwareInterface, AUDIO_SAMPLE_RATE, NUMBER_OF_AUDIO_CHANNELS,
    NUMBER_OF_AUDIO_SAMPLES,
};

#[cfg(feature = "nao")]
pub use nao::NaoInterface;

#[cfg(feature = "webots")]
pub use self::webots::WebotsInterface;
