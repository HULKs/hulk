use std::fmt::{self, Display, Formatter};

use enum_iterator::Sequence;
use serde::{Deserialize, Serialize};

/// Robot-local, event-only topic shared by sound request publishers and the sound handler.
pub const SOUND_REQUEST_TOPIC: &str = "commands/speaker_request";
pub const SOUNDBOARD_TOPIC: &str = "soundboard";

#[derive(Copy, Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize, ros_z::Message)]
#[serde(deny_unknown_fields)]
pub struct Soundboard {
    pub ball: bool,
    pub donk: bool,
    pub drift: bool,
    pub game_controller_collision: bool,
    pub metal_bar: bool,
    pub ouch: bool,
    pub sigh: bool,
    pub weeeee: bool,
}

impl Soundboard {
    pub fn is_enabled(&self, sound: Sound) -> bool {
        match sound {
            Sound::Ball => self.ball,
            Sound::Donk => self.donk,
            Sound::Drift => self.drift,
            Sound::GameControllerCollision => self.game_controller_collision,
            Sound::MetalBar => self.metal_bar,
            Sound::Ouch => self.ouch,
            Sound::Sigh => self.sigh,
            Sound::Weeeee => self.weeeee,
        }
    }
}

#[derive(Copy, Clone, Debug, Deserialize, Eq, PartialEq, Serialize, ros_z::Message)]
pub enum SpeakerRequest {
    PlaySound { sound: Sound },
}

#[derive(
    Copy, Clone, Debug, Deserialize, Eq, Hash, PartialEq, Sequence, Serialize, ros_z::Message,
)]
pub enum Sound {
    Ball,
    Donk,
    Drift,
    GameControllerCollision,
    MetalBar,
    Ouch,
    Sigh,
    Weeeee,
}

impl Display for Sound {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        write!(formatter, "{self:?}")
    }
}
