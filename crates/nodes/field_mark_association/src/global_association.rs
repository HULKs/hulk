mod associator;
mod config;
mod map;
mod solver;
#[cfg(test)]
mod tests;
mod types;

pub(crate) use associator::{
    GlobalAssociator, GlobalLocalizationInput, GlobalLocalizationResult, PoseHintAssociationResult,
};
pub(crate) use config::GLOBAL_LOCALIZER_MAX_DETECTIONS;
pub use config::{GlobalAssociationConfig, PoseHintAssociationConfig};
pub(crate) use types::{FEATURE_CLASSES, FeatureAssociation, FeatureAssociations};
pub use types::{
    GlobalLocalizationDebugAssociation, GlobalLocalizationDebugDetection,
    GlobalLocalizationDebugProjection, GlobalLocalizationDetailedDebug,
    GlobalLocalizationDetailedStatus, GlobalLocalizationScore, VisualFeatureClass,
};
