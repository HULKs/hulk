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
        if other.y() < self.leftmost_point.y() {
            self.leftmost_point = other;
        } else if other.y() > self.rightmost_point.y() {
            self.rightmost_point = other;
        }
    }
}
