use coordinate_systems::Ground;
use linear_algebra::{IntoFramed, Point2, Vector2};
use types::detected_feet::CountedCluster;

pub trait MeanClustering {
    fn push(&mut self, other: Point2<Ground>);
    fn mean(&self) -> Point2<Ground>;
    fn standard_deviation(&self) -> Vector2<Ground>;
}

impl MeanClustering for CountedCluster {
    fn push(&mut self, other: Point2<Ground>) {
        self.sum += other.coords();
        self.sum_squared += other.map(|x| x * x).coords();
        self.samples += 1;

        if other.x() < self.leftmost_point.x() {
            self.leftmost_point = other;
        } else if other.x() > self.rightmost_point.x() {
            self.rightmost_point = other;
        }
    }

    fn mean(&self) -> Point2<Ground> {
        self.sum / self.samples as f32
    }

    #[doc = "Computes the individual standard deviations of the x and y components of the cluster."]
    fn standard_deviation(&self) -> Vector2<Ground> {
        ((self.sum_squared / self.samples as f32) - (self.sum / self.samples as f32).map(|x| x * x))
            .inner
            .map(|x| x.sqrt())
            .framed()
    }
}
