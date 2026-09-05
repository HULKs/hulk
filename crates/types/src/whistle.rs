use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Default, Deserialize, Serialize, ros_z::Message)]
pub struct Whistle {
    pub is_detected: Vec<bool>,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize, ros_z::Message)]
pub struct DetectionInfo {
    pub overall_mean: f32,
    pub std_deviation: f32,
    pub background_noise_threshold: f32,
    pub whistle_threshold: f32,
    pub min_frequency_index: usize,
    pub max_frequency_index: usize,
    pub band_size: usize,
    pub chunk_size: usize,
    pub whistle_mean: Option<f32>,
    pub band_mean: f32,
    pub lower_whistle_chunk: Option<usize>,
    pub upper_whistle_chunk: Option<usize>,
    pub lower_band_index: Option<usize>,
    pub upper_band_index: Option<usize>,
}

#[derive(Clone, Copy, Debug, Default, Deserialize, Serialize, ros_z::Message)]
pub struct AudioSpectrumBin {
    pub frequency: f32,
    pub magnitude: f32,
}

pub type AudioSpectrum = Vec<AudioSpectrumBin>;
