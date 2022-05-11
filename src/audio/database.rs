use crate::types::Whistle;
use macros::SerializeHierarchy;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Default, SerializeHierarchy)]
pub struct MainOutputs {
    pub detected_whistle: Option<Whistle>,
}

#[derive(Debug, Default, Clone, SerializeHierarchy)]
pub struct AdditionalOutputs {
    pub audio_spectrums: Option<Vec<Vec<(f32, f32)>>>,
    pub detection_infos: Option<Vec<DetectionInfo>>,
}

#[derive(Debug, Default, Clone, SerializeHierarchy, Serialize, Deserialize)]
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

#[derive(Debug, Default, Clone)]
pub struct Database {
    pub main_outputs: MainOutputs,
    pub additional_outputs: AdditionalOutputs,
}
