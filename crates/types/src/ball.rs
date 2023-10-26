use approx_derive::{AbsDiffEq, RelativeEq};
use geometry::circle::Circle;
use nalgebra::Point2;
use serde::{Deserialize, Serialize};
use serialize_hierarchy::SerializeHierarchy;

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Serialize, SerializeHierarchy)]
pub struct CandidateEvaluation {
    pub candidate_circle: Circle,
    pub preclassifier_confidence: f32,
    pub classifier_confidence: Option<f32>,
    pub corrected_circle: Option<Circle>,
    pub merge_weight: Option<f32>,
}

#[derive(
    Clone, Debug, Deserialize, PartialEq, Serialize, SerializeHierarchy, AbsDiffEq, RelativeEq,
)]
#[abs_diff_eq(epsilon = "f32")]
pub struct Ball {
    pub position: Point2<f32>,
    pub image_location: Circle,
}
