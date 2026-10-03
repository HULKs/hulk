use coordinate_systems::{Camera, Ground, ImuReference, Robot};
use linear_algebra::{Isometry3, Orientation2, Orientation3, Rotation3};
use projection::intrinsic::Intrinsic;
use ros_z::time::Time;
pub use types::localization::HeadingConstraint;
use types::{
    field_dimensions::FieldDimensions,
    localization::{LocalizationState, LocalizationStatus},
    visual_localization::{
        AssociationGeometry, FieldMarkAssociation, GlobalLocalizationDebug, VisualAssociationSource,
    },
};

use crate::{
    DetectedVisualFeatures, FieldMarkAssociationParameters, GlobalLocalizerParameters,
    global_association::solver, tracking,
};

/// Exposure-time sensor inputs and an optional coherent tracking snapshot.
#[derive(Clone, Copy)]
pub struct AssociationInput<'a> {
    pub status: &'a LocalizationStatus,
    pub attitude: Option<Orientation3<ImuReference>>,
    pub tracking: Option<&'a AssociationGeometry>,
    pub visual_features: &'a DetectedVisualFeatures,
    pub robot_to_camera: Isometry3<Robot, Camera>,
    pub camera_intrinsic: Intrinsic,
    pub field_dimensions: &'a FieldDimensions,
    pub time: Time,
}

/// Pose prediction for an established Tracking state.
#[derive(Clone, Copy)]
pub struct TrackingAssociationInput<'a> {
    pub visual_features: &'a DetectedVisualFeatures,
    pub robot_to_camera: Isometry3<Robot, Camera>,
    pub geometry: &'a AssociationGeometry,
    pub camera_intrinsic: Intrinsic,
    pub field_dimensions: &'a FieldDimensions,
    pub time: Time,
}

#[derive(Clone, Copy)]
pub struct GlobalAssociationInput<'a> {
    pub visual_features: &'a DetectedVisualFeatures,
    /// Exposure-time leveling rotation, with robot heading removed.
    pub robot_to_ground: Rotation3<Robot, Ground>,
    pub robot_to_camera: Isometry3<Robot, Camera>,
    pub camera_intrinsic: Intrinsic,
    pub field_dimensions: &'a FieldDimensions,
    pub parameters: &'a GlobalLocalizerParameters,
    pub heading: Option<HeadingConstraint>,
}

/// Fixed correspondences only; pose estimation belongs to localization.
#[derive(Default)]
pub struct AssociationResult {
    pub associations: Vec<FieldMarkAssociation>,
    pub source: VisualAssociationSource,
    pub debug: Option<GlobalLocalizationDebug>,
}

/// Shared lifecycle dispatch for the robot and simulator. Global matching uses
/// sensor attitude at exposure; only tracking consumes optimizer geometry.
pub fn associate_visual_features(
    input: AssociationInput<'_>,
    parameters: &FieldMarkAssociationParameters,
) -> AssociationResult {
    if parameters.validate().is_err() {
        return AssociationResult::default();
    }
    if input.status.state == LocalizationState::Tracking {
        let Some(geometry) = input.tracking.filter(|geometry| {
            geometry.epoch == input.status.epoch && geometry.generation == input.status.generation
        }) else {
            return AssociationResult::default();
        };
        return tracking::associate(
            TrackingAssociationInput {
                geometry,
                visual_features: input.visual_features,
                robot_to_camera: input.robot_to_camera,
                camera_intrinsic: input.camera_intrinsic,
                field_dimensions: input.field_dimensions,
                time: input.time,
            },
            parameters,
        )
        .unwrap_or_default();
    }
    let Some(attitude) = input.attitude else {
        return AssociationResult::default();
    };
    let (roll, pitch, yaw) = attitude.euler_angles();
    let heading = if input.status.state == LocalizationState::LostTrack {
        if input.time <= input.status.time {
            return AssociationResult::default();
        }
        let Some(heading) = input
            .status
            .heading
            .and_then(|reference| reference.at(input.time, Orientation2::new(f64::from(yaw))))
        else {
            return AssociationResult::default();
        };
        Some(heading)
    } else {
        None
    };
    associate_global_visual_features(GlobalAssociationInput {
        visual_features: input.visual_features,
        robot_to_ground: Rotation3::from_euler_angles(roll, pitch, 0.0),
        robot_to_camera: input.robot_to_camera,
        camera_intrinsic: input.camera_intrinsic,
        field_dimensions: input.field_dimensions,
        parameters: &parameters.global_localizer,
        heading,
    })
}

/// Predict map landmarks with the supplied pose prior, independent of node lifecycle.
/// Sparse frames use the joint pixel Gaussian; larger frames use marginal assignment.
pub fn associate_tracking_visual_features(
    input: TrackingAssociationInput<'_>,
    parameters: &FieldMarkAssociationParameters,
) -> AssociationResult {
    if parameters.validate().is_err() {
        return AssociationResult::default();
    }
    tracking::associate(input, parameters).unwrap_or_default()
}

/// Reject ambiguous assignments and exhausted budgets. Heading constrains oriented
/// assignments; without it, uniqueness is modulo field half-turn symmetry.
pub fn associate_global_visual_features(input: GlobalAssociationInput<'_>) -> AssociationResult {
    solver::associate(input).unwrap_or_default()
}
