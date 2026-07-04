use coordinate_systems::{Ground, Pixel};
use geometry::circle::Circle;
use linear_algebra::Point2;
use nalgebra::Matrix2;

#[derive(Clone, Copy, Debug)]
pub struct Measurement {
    pub position: Point2<Ground>,
    pub covariance: Matrix2<f32>,
    pub confidence: f32,
    pub image_location: Option<Circle<Pixel>>,
}

impl Measurement {
    pub fn confidence_likelihood(&self) -> f32 {
        self.confidence.clamp(0.01, 1.0).ln()
    }
}
