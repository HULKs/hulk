use ros_z::Message;
use serde::{Deserialize, Serialize};

use coordinate_systems::Pixel;
use geometry::circle::Circle;

use crate::multivariate_normal_distribution::MultivariateNormalDistribution;

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Serialize, Message)]
pub struct CandidateEvaluation {
    pub candidate_circle: Circle<Pixel>,
    pub preclassifier_confidence: f32,
    pub classifier_confidence: Option<f32>,
    pub corrected_circle: Option<Circle<Pixel>>,
    pub merge_weight: Option<f32>,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, Message)]
pub struct BallPercept {
    pub percept_in_ground: MultivariateNormalDistribution<2>,
    pub image_location: Circle<Pixel>,
}
