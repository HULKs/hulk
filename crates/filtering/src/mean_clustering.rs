use coordinate_systems::{Framed, IntoFramed};
use nalgebra::Point2;
use types::coordinate_systems::Ground;
use types::detected_feet::CountedCluster;

pub trait MeanClustering {
    fn push(&mut self, other: Framed<Ground, Point2<f32>>);
}

impl MeanClustering for CountedCluster {
    fn push(&mut self, other: Framed<Ground, Point2<f32>>) {
        self.mean = ((self.mean.inner * self.samples as f32 + other.inner.coords)
            / (self.samples + 1) as f32)
            .framed();
        self.samples += 1;
    }
}
