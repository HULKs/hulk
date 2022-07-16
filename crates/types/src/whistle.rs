use serde::{Deserialize, Serialize};
use serialize_hierarchy::SerializeHierarchy;

#[derive(Clone, Debug, Default, Deserialize, Serialize, SerializeHierarchy)]
pub struct Whistle {
    pub is_detected: Vec<bool>,
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
