use coordinate_systems::{Field, Ground, Pixel, Robot};
use linear_algebra::{Isometry3, Point2};

use super::GlobalLocalizationResult;

pub(crate) const FEATURE_CLASSES: [VisualFeatureClass; 5] = [
    VisualFeatureClass::GoalPost,
    VisualFeatureClass::LSpot,
    VisualFeatureClass::TSpot,
    VisualFeatureClass::XSpot,
    VisualFeatureClass::PenaltySpot,
];

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
/// Field-feature classes supported by the global association solver.
pub enum VisualFeatureClass {
    /// Upright goalpost landmark detected at its field-contact point.
    GoalPost,
    /// L-shaped line crossing landmark.
    LSpot,
    /// T-shaped line crossing landmark.
    TSpot,
    /// X-shaped line crossing landmark.
    XSpot,
    /// Penalty marker landmark.
    PenaltySpot,
}

#[derive(Clone, Debug)]
pub(crate) struct FeatureAssociations {
    pub robot_to_field: Isometry3<Robot, Field>,
    pub features: Vec<FeatureAssociation>,
    pub score: GlobalLocalizationScore,
}

#[derive(Clone, Debug)]
pub(crate) struct FeatureAssociation {
    pub detection_id: usize,
    pub landmark_id: usize,
    pub detection: Point2<Pixel>,
    pub field_point: Point2<Field>,
}

#[derive(Clone, Copy, Debug)]
/// Scores and residual summaries for a global localization hypothesis.
pub struct GlobalLocalizationScore {
    /// Accepted fixed feature association count.
    pub inliers: usize,
    /// Weighted internal candidate score used by `min_score` and `score_ratio`.
    pub candidate_score: f32,
    /// Metric field-space RMS residual used by `rms_threshold`.
    pub metric_rms_residual: f32,
    /// Pixel reprojection RMS for the selected pose and fixed associations.
    pub reprojection_rmse: f32,
    /// Sum of squared pixel reprojection errors for the selected pose.
    pub total_cost: f32,
}

#[derive(Clone, Debug)]
/// Detailed per-feature debug payload for visualizing a global localization result.
pub struct GlobalLocalizationDetailedDebug {
    /// Certification status for the returned associations.
    pub status: GlobalLocalizationDetailedStatus,
    /// Selected robot pose in the field frame.
    pub robot_to_field: Isometry3<Robot, Field>,
    /// Candidate score and residual summary for the selected result.
    pub score: GlobalLocalizationScore,
    /// Back-projected detections considered by the solver.
    pub detections: Vec<GlobalLocalizationDebugDetection>,
    /// Field landmarks projected into the image for the selected pose.
    pub projected_features: Vec<GlobalLocalizationDebugProjection>,
    /// Accepted detection-to-landmark associations for the selected result.
    pub associations: Vec<GlobalLocalizationDebugAssociation>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
/// Certification status for detailed global-localization debug output.
pub enum GlobalLocalizationDetailedStatus {
    /// Returned stable associations are unique after quotienting the unavoidable 180 degree field
    /// symmetry.
    UniqueModuloSymmetry,
}

#[derive(Clone, Debug)]
/// Debug record for one field-feature detection considered by global localization.
pub struct GlobalLocalizationDebugDetection {
    /// Stable detection index from the raw feature stream.
    pub index: usize,
    /// Landmark class inferred from the detector output.
    pub class: VisualFeatureClass,
    /// Input image point used by projection and association.
    pub pixel: Point2<Pixel>,
    /// Back-projected ground point under the current camera geometry.
    pub ground: Point2<Ground>,
}

#[derive(Clone, Debug)]
/// Debug record for one known field landmark projected into the current camera image.
pub struct GlobalLocalizationDebugProjection {
    /// Landmark identifier in the generated field map.
    pub index: usize,
    /// Identifier of the landmark reached by 180-degree field symmetry.
    pub symmetric_index: usize,
    /// Landmark class used for same-class association.
    pub class: VisualFeatureClass,
    /// Landmark position in field coordinates.
    pub field_point: Point2<Field>,
    /// Pixel projection for the selected pose, or `None` if the landmark is behind the camera.
    pub projected_pixel: Option<Point2<Pixel>>,
    /// Whether this landmark is part of the accepted association set.
    pub accepted: bool,
}

#[derive(Clone, Debug)]
/// Debug record for one accepted detection-to-landmark association.
pub struct GlobalLocalizationDebugAssociation {
    /// Stable detection index from the raw feature stream.
    pub detection_index: usize,
    /// Landmark identifier in the generated field map.
    pub feature_index: usize,
    /// Landmark class shared by the detection and field point.
    pub class: VisualFeatureClass,
    /// Input detection pixel.
    pub detection_pixel: Point2<Pixel>,
    /// Detection back-projected onto the ground plane.
    pub back_projected_ground: Point2<Ground>,
    /// Associated landmark position in field coordinates.
    pub field_point: Point2<Field>,
    /// Pixel projection of the associated landmark for the selected pose.
    pub projected_pixel: Option<Point2<Pixel>>,
    /// Pixel distance between `detection_pixel` and `projected_pixel`, if projected.
    pub reprojection_error_px: Option<f32>,
}

impl GlobalLocalizationResult {
    /// Returns the best association set carried by this result, regardless of certification status.
    pub fn associations(&self) -> &FeatureAssociations {
        match self {
            Self::UniqueModuloSymmetry(associations) => associations,
        }
    }

    #[cfg(test)]
    /// Returns whether this test result is certified unique modulo field symmetry.
    pub fn is_unique(&self) -> bool {
        matches!(self, Self::UniqueModuloSymmetry(_))
    }

    pub(crate) fn unique_feature_associations(&self) -> &[FeatureAssociation] {
        match self {
            Self::UniqueModuloSymmetry(associations) => &associations.features,
        }
    }
}
