use approx::{AbsDiffEq, RelativeEq};
use nalgebra::Point2;
use serde::{Deserialize, Serialize};
use serialize_hierarchy::SerializeHierarchy;

use super::Circle;

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Serialize, SerializeHierarchy)]
pub struct CandidateEvaluation {
    pub grid_element: Circle,
    pub preclassifier_confidence: f32,
    pub classifier_confidence: Option<f32>,
    pub positioned_ball: Option<Circle>,
    pub positioned_robot: Option<Circle>, // needs own datatype
    pub merge_weight: Option<f32>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize, SerializeHierarchy)]
pub struct Ball {
    pub position: Point2<f32>,
    pub image_location: Circle,
}

impl AbsDiffEq for Ball {
    type Epsilon = f32;

    fn default_epsilon() -> Self::Epsilon {
        Self::Epsilon::default_epsilon()
    }

    fn abs_diff_eq(&self, other: &Self, epsilon: Self::Epsilon) -> bool {
        self.position.abs_diff_eq(&other.position, epsilon)
            && self
                .image_location
                .abs_diff_eq(&other.image_location, epsilon)
    }
}

impl RelativeEq for Ball {
    fn default_max_relative() -> Self::Epsilon {
        Self::Epsilon::default_max_relative()
    }

    fn relative_eq(
        &self,
        other: &Self,
        epsilon: Self::Epsilon,
        max_relative: Self::Epsilon,
    ) -> bool {
        self.position
            .relative_eq(&other.position, epsilon, max_relative)
            && self
                .image_location
                .relative_eq(&other.image_location, epsilon, max_relative)
    }
}
