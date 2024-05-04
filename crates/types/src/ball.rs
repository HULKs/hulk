use serde::{Deserialize, Serialize};

use approx_derive::{AbsDiffEq, RelativeEq};
use coordinate_systems::{Ground, Pixel};
use geometry::circle::Circle;
use linear_algebra::Point2;
use path_serde::{PathDeserialize, PathIntrospect, PathSerialize};

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Serialize, PathSerialize, PathIntrospect)]
pub struct CandidateEvaluation {
    pub candidate_circle: Circle<Pixel>,
    pub preclassifier_confidence: f32,
    pub classifier_confidence: Option<f32>,
    pub corrected_circle: Option<Circle<Pixel>>,
    pub merge_weight: Option<f32>,
}

#[derive(
    Clone,
    Debug,
    Deserialize,
    Serialize,
    PathSerialize,
    PathDeserialize,
    PathIntrospect,
    AbsDiffEq,
    RelativeEq,
    PartialEq,
)]
#[abs_diff_eq(epsilon_type = f32)]
pub struct Ball {
    pub position: Point2<Ground>,
    pub image_location: Circle<Pixel>,
}
