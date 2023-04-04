use nalgebra::Point2;
use types::detected_feet::CountedCluster;

pub trait MeanClustering {
    fn push(&mut self, other: Point2<f32>);
}

impl MeanClustering for CountedCluster {
    fn push(&mut self, other: Point2<f32>) {
        self.mean = (self.mean * self.samples as f32 + other.coords) / (self.samples + 1) as f32;
        self.samples += 1;
    }
}
