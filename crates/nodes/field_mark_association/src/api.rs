use coordinate_systems::{Field, Robot};
use linear_algebra::Isometry3;
use projection::camera_matrix::CameraMatrix;
use types::{field_dimensions::FieldDimensions, visual_localization::FieldMarkAssociation};

use crate::{
    debug::global_localization_debug_from_result,
    features::DetectedVisualFeatures,
    global_association::{
        FeatureAssociation, GlobalAssociationConfig as GlobalLocalizerParameters, GlobalAssociator,
        GlobalLocalizationDetailedDebug, GlobalLocalizationInput, GlobalLocalizationResult,
    },
    parameters::FieldMarkAssociationParameters,
    robot_to_camera,
    tracking::FieldMarkAssociationState,
};

/// Result of running visual global localization on one object-detection frame.
pub struct GlobalVisualLocalization {
    /// Debug payload for the best visual global localization result, if any.
    pub debug: Option<types::visual_localization::GlobalLocalizationDebug>,
    /// Accepted global pose when global recovery should reset the backend state.
    pub accepted_global_pose: Option<Isometry3<Robot, Field>>,
    /// Fixed associations selected by either global uniqueness or pose-hint fallback.
    pub associations: Vec<FieldMarkAssociation>,
}

/// Runs global localization first and falls back to pose-hint association when needed.
pub fn associate_visual_features(
    visual_features: &DetectedVisualFeatures,
    camera_matrix: &CameraMatrix,
    field_dimensions: &FieldDimensions,
    pose_hint: Option<Isometry3<Robot, Field>>,
    parameters: &FieldMarkAssociationParameters,
) -> GlobalVisualLocalization {
    FieldMarkAssociationState::default().associate_visual_features_with_debug(
        visual_features,
        camera_matrix,
        field_dimensions,
        pose_hint,
        parameters,
        true,
    )
}

/// Runs global localization and returns debug data plus backend-safe associations.
pub fn localize_global_visual_features(
    visual_features: &DetectedVisualFeatures,
    camera_matrix: &CameraMatrix,
    field_dimensions: &FieldDimensions,
    pose_hint: Option<Isometry3<Robot, Field>>,
    parameters: &GlobalLocalizerParameters,
) -> GlobalVisualLocalization {
    localize_global_visual_features_with_debug(
        visual_features,
        camera_matrix,
        field_dimensions,
        pose_hint,
        parameters,
        true,
    )
}

fn localize_global_visual_features_with_debug(
    visual_features: &DetectedVisualFeatures,
    camera_matrix: &CameraMatrix,
    field_dimensions: &FieldDimensions,
    pose_hint: Option<Isometry3<Robot, Field>>,
    parameters: &GlobalLocalizerParameters,
    include_debug: bool,
) -> GlobalVisualLocalization {
    let localizer = GlobalAssociator::new(*parameters);
    let result = localizer.localize(GlobalLocalizationInput {
        visual_features,
        field_dimensions,
        ground_to_robot: camera_matrix.ground_to_robot,
        robot_to_camera: robot_to_camera(camera_matrix),
        camera_intrinsic: camera_matrix.intrinsics,
        pose_hint,
    });

    GlobalVisualLocalization {
        debug: if include_debug {
            result.as_ref().map(global_localization_debug_from_result)
        } else {
            None
        },
        accepted_global_pose: None,
        associations: result
            .as_ref()
            .map(GlobalLocalizationResult::unique_feature_associations)
            .map(|associations| {
                field_mark_associations(
                    associations.iter(),
                    types::visual_localization::FieldMarkAssociationSource::GlobalUnique,
                )
            })
            .unwrap_or_default(),
    }
}

pub(crate) fn field_mark_associations<'a>(
    associations: impl IntoIterator<Item = &'a FeatureAssociation>,
    source: types::visual_localization::FieldMarkAssociationSource,
) -> Vec<FieldMarkAssociation> {
    associations
        .into_iter()
        .map(|association| FieldMarkAssociation {
            detection: association.detection,
            field_point: association.field_point.extend(0.0),
            source,
        })
        .collect()
}

/// Runs global localization and returns per-feature debug data for visual inspection.
pub fn localize_global_visual_features_detailed_debug(
    visual_features: &DetectedVisualFeatures,
    camera_matrix: &CameraMatrix,
    field_dimensions: &FieldDimensions,
    pose_hint: Option<Isometry3<Robot, Field>>,
    parameters: &GlobalLocalizerParameters,
) -> Option<GlobalLocalizationDetailedDebug> {
    let localizer = GlobalAssociator::new(*parameters);
    localizer.localize_detailed(GlobalLocalizationInput {
        visual_features,
        field_dimensions,
        ground_to_robot: camera_matrix.ground_to_robot,
        robot_to_camera: robot_to_camera(camera_matrix),
        camera_intrinsic: camera_matrix.intrinsics,
        pose_hint,
    })
}
