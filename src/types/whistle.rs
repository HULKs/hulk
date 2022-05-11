use macros::SerializeHierarchy;
use serde::{Deserialize, Serialize};

use crate::hardware::NUMBER_OF_AUDIO_CHANNELS;

#[derive(Clone, Debug, Default, Deserialize, Serialize, SerializeHierarchy)]
pub struct Whistle {
    #[leaf]
    pub is_detected: [bool; NUMBER_OF_AUDIO_CHANNELS],
}
