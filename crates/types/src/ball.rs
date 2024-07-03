use serde::{Deserialize, Serialize};

use coordinate_systems::Pixel;
use geometry::circle::Circle;
use path_serde::{PathDeserialize, PathIntrospect, PathSerialize};

use crate::multivariate_normal_distribution::MultivariateNormalDistribution;

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Serialize, PathSerialize, PathIntrospect)]
pub struct CandidateEvaluation {
    pub candidate_circle: Circle<Pixel>,
    pub preclassifier_confidence: f32,
    pub classifier_confidence: Option<f32>,
    pub corrected_circle: Option<Circle<Pixel>>,
    pub merge_weight: Option<f32>,
}

#[derive(Clone, Debug, Deserialize, Serialize, PathSerialize, PathDeserialize, PathIntrospect)]
pub struct BallDetection {
    pub detection: MultivariateNormalDistribution<2>,
    pub image_location: Circle<Pixel>,
}
