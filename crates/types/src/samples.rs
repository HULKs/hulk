use std::sync::Arc;

#[derive(Clone, Debug, Default)]
pub struct Samples {
    pub rate: u32,
    pub channels_of_samples: Arc<Vec<Vec<f32>>>,
}
