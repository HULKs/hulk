use coordinate_systems::Ground;
use linear_algebra::Point2;
use types::detected_feet::CountedCluster;

pub trait MeanClustering {
    fn push(&mut self, other: Point2<Ground>);
}

impl MeanClustering for CountedCluster {
    fn push(&mut self, other: Point2<Ground>) {
        self.mean = (self.mean * self.samples as f32 + other.coords()) / (self.samples + 1) as f32;
        self.samples += 1;
        if other.x() < self.lefmost_point.x() {
            self.lefmost_point = other;
        } else if other.x() > self.rightmost_point.x() {
            self.rightmost_point = other;
        }
    }
}
