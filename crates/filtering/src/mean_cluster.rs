use nalgebra::Point2;
use serde::{Deserialize, Serialize};
use serialize_hierarchy::SerializeHierarchy;

#[derive(Default, Clone, Debug, Deserialize, Serialize, SerializeHierarchy)]
pub struct MeanCluster {
    pub mean: Point2<f32>,
    pub samples: usize,
}

impl MeanCluster {
    pub fn new(mean: Point2<f32>) -> Self {
        Self { mean, samples: 1 }
    }

    pub fn push(&mut self, other: Point2<f32>) {
        self.mean = (self.mean * self.samples as f32 + other.coords) / (self.samples + 1) as f32;
        self.samples += 1;
    }
}
