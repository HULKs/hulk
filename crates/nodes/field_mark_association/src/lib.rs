use coordinate_systems::{Camera, Robot};
use linear_algebra::Isometry3;
use projection::camera_matrix::CameraMatrix;

mod api;
mod debug;
mod features;
mod frame_processing;
mod global_association;
mod node;
mod parameters;
mod tracking;

pub use api::{
    GlobalVisualLocalization, associate_visual_features, localize_global_visual_features,
    localize_global_visual_features_detailed_debug,
};
pub use features::{
    DetectedVisualFeature, DetectedVisualFeatures, find_detected_goalposts,
    find_detected_visual_features,
};
pub use global_association::{
    GlobalAssociationConfig as GlobalLocalizerParameters, GlobalLocalizationDebugAssociation,
    GlobalLocalizationDebugDetection, GlobalLocalizationDebugProjection,
    GlobalLocalizationDetailedDebug, GlobalLocalizationDetailedStatus, GlobalLocalizationScore,
    PoseHintAssociationConfig as PoseHintAssociationParameters, VisualFeatureClass,
};
pub use node::{run, run_boxed};
pub use parameters::FieldMarkAssociationParameters;
pub use tracking::FieldMarkAssociationState;
pub use types::visual_localization::{
    FieldMarkAssociation, FieldMarkAssociationSource,
    FieldMarkAssociationSource as FieldMarkAssociationKind, GLOBAL_LOCALIZATION_DEBUG_TOPIC,
    GlobalLocalizationDebug, GlobalLocalizationDebugStatus, VisualLocalizationFrame,
    VisualLocalizationFrame as FieldMarkAssociations,
};

pub(crate) fn robot_to_camera(camera_matrix: &CameraMatrix) -> Isometry3<Robot, Camera> {
    camera_matrix.head_to_camera * camera_matrix.robot_to_head
}
