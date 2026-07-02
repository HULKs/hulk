use std::{
    future::{Future, ready},
    pin::Pin,
    sync::Arc,
    time::Duration,
};

use color_eyre::{Result, eyre::Context as _};
use coordinate_systems::{Camera, Field, Pixel, Robot};
use global_association::{
    FeatureAssociation, GlobalAssociator, GlobalLocalizationInput, GlobalLocalizationResult,
    PoseHintAssociationResult,
};
use linear_algebra::{Isometry3, Point2, Point3, point};
use projection::camera_matrix::CameraMatrix;
use ros_z::{
    Message,
    context::Context,
    parameter::NodeParametersExt,
    qos::{QosDurability, QosProfile},
};
use ros_z_streams::CreateFutureMapBuilder;
use serde::{Deserialize, Serialize};
use types::{
    field_dimensions::FieldDimensions,
    object_detection::{Object, RobocupObjectLabel},
    time_wrapper::TimeWrapper,
};

mod global_association;

pub use global_association::{
    GlobalAssociationConfig as GlobalLocalizerParameters, GlobalLocalizationDebugAssociation,
    GlobalLocalizationDebugDetection, GlobalLocalizationDebugProjection,
    GlobalLocalizationDetailedDebug, GlobalLocalizationDetailedStatus, GlobalLocalizationScore,
    PoseHintAssociationConfig as PoseHintAssociationParameters, VisualFeatureClass,
};

const MAX_CAMERA_MATRIX_TIME_DISTANCE: Duration = Duration::from_millis(100);
const DETECTED_OBJECTS_SAFETY_LAG: Duration = Duration::from_millis(50);

#[derive(Clone, Default, Debug, Deserialize, Serialize, Message)]
#[serde(deny_unknown_fields)]
pub struct FieldMarkAssociationParameters {
    pub global_localizer: GlobalLocalizerParameters,
    pub pose_hint: PoseHintAssociationParameters,
}

impl FieldMarkAssociationParameters {
    fn validate(&self) -> std::result::Result<(), String> {
        self.global_localizer.validate()?;
        self.pose_hint.validate()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Message)]
pub struct FieldMarkAssociation {
    pub detection: Point2<Pixel>,
    pub field_point: Point3<Field>,
    pub kind: FieldMarkAssociationKind,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, Serialize, Deserialize, Message)]
pub enum FieldMarkAssociationKind {
    GlobalUnique,
    PoseHint,
}

#[derive(Debug, Clone, Serialize, Deserialize, Message)]
pub struct FieldMarkAssociations {
    pub robot_to_camera: Isometry3<Robot, Camera>,
    pub associations: Vec<FieldMarkAssociation>,
}

/// Result of running visual global localization on one object-detection frame.
pub struct GlobalVisualLocalization {
    /// Debug payload for the best visual global localization result, if any.
    pub debug: Option<GlobalLocalizationDebug>,
    /// Accepted global pose when global recovery should reset the backend state.
    pub accepted_global_pose: Option<Isometry3<Robot, Field>>,
    /// Fixed associations selected by either global uniqueness or pose-hint fallback.
    pub associations: Vec<FieldMarkAssociation>,
}

#[derive(Debug, Clone, Default)]
pub struct FieldMarkAssociationState {
    pending_global_recovery: Option<GlobalRecoveryCandidate>,
    has_global_lock: bool,
}

#[derive(Debug, Clone)]
struct GlobalRecoveryCandidate {
    robot_to_field: Isometry3<Robot, Field>,
    associations: Vec<FieldMarkAssociation>,
    consecutive_frames: usize,
}

struct AcceptedFieldMarkAssociations {
    associations: Vec<FieldMarkAssociation>,
    global_pose: Option<Isometry3<Robot, Field>>,
}

impl AcceptedFieldMarkAssociations {
    fn empty() -> Self {
        Self {
            associations: Vec::new(),
            global_pose: None,
        }
    }

    fn associations(associations: Vec<FieldMarkAssociation>) -> Self {
        Self {
            associations,
            global_pose: None,
        }
    }

    fn global(
        robot_to_field: Isometry3<Robot, Field>,
        associations: Vec<FieldMarkAssociation>,
    ) -> Self {
        Self {
            associations,
            global_pose: Some(robot_to_field),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Message)]
/// Published debug data for a successful global localization hypothesis.
pub struct GlobalLocalizationDebug {
    /// Best robot pose in the field frame for this visual result.
    pub robot_to_field: Isometry3<Robot, Field>,
    /// Whether the best result is ambiguous or unique modulo field symmetry.
    pub status: GlobalLocalizationDebugStatus,
    /// Number of fixed feature associations accepted by the global-localizer gates.
    pub inliers: usize,
    /// Root-mean-square reprojection error in pixels.
    pub reprojection_rmse: f32,
    /// Sum of squared reprojection errors in pixels squared.
    pub total_cost: f32,
    /// Weighted internal candidate score used by `min_score` and `score_ratio`.
    pub candidate_score: f32,
    /// Metric field-space RMS residual used by `rms_threshold`.
    pub metric_rms_residual: f32,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, Message)]
/// Classification of a successful global localization result.
pub enum GlobalLocalizationDebugStatus {
    /// Uniqueness was not certified because a competitor may remain or the bounded search ended
    /// inconclusively.
    Ambiguous,
    /// The assignment is unique after quotienting the unavoidable 180 degree
    /// field symmetry. The chosen branch follows the pose hint when available.
    UniqueModuloSymmetry,
}

/// Starts the field-mark association node and erases the concrete future type for node runners.
pub fn run_boxed(ctx: Arc<Context>) -> Pin<Box<dyn Future<Output = Result<()>> + Send>> {
    Box::pin(run(ctx))
}

pub async fn run(ctx: Arc<Context>) -> Result<()> {
    let node = ctx.create_node("field_mark_association").build().await?;
    let parameters =
        node.bind_parameter_as::<FieldMarkAssociationParameters>("field_mark_association")?;
    parameters.add_validation_hook(FieldMarkAssociationParameters::validate)?;

    let camera_matrix_cache = node
        .subscriber::<TimeWrapper<CameraMatrix>>("camera_matrix")
        .cache(128)
        .with_stamp(|message| message.time)
        .build()
        .await?;

    let field_dimensions_cache = node
        .subscriber::<FieldDimensions>("field_dimensions")
        .qos(QosProfile {
            durability: QosDurability::TransientLocal,
            ..Default::default()
        })
        .cache(1)
        .build()
        .await?;

    let localization_cache = node
        .subscriber::<TimeWrapper<Option<Isometry3<Field, Robot>>>>("localization/timestamped")
        .cache(128)
        .with_stamp(|message| message.time)
        .build()
        .await?;

    let mut detected_objects = node
        .create_future_map_builder()
        .create_future_subscriber::<Vec<Object<RobocupObjectLabel>>>(
            "detected_objects",
            DETECTED_OBJECTS_SAFETY_LAG,
        )
        .await?
        .build();

    let associations_publisher = node
        .publisher::<TimeWrapper<FieldMarkAssociations>>("field_mark_association/associations")
        .build()
        .await?;
    let global_localization_publisher = node
        .publisher::<Option<GlobalLocalizationDebug>>("debug/global_localization")
        .build()
        .await?;
    let global_pose_publisher = node
        .publisher::<TimeWrapper<Option<Isometry3<Robot, Field>>>>(
            "field_mark_association/global_pose",
        )
        .build()
        .await?;
    let mut association_state = FieldMarkAssociationState::default();

    loop {
        let item = detected_objects.recv().await?;
        for (image_time, (objects,)) in item.persistent {
            let Some(camera_matrix) = camera_matrix_cache.get_nearest(image_time) else {
                continue;
            };
            if !camera_matrix_is_fresh(&camera_matrix, image_time) {
                continue;
            }
            let Some(field_dimensions) = field_dimensions_cache.get_nearest(image_time) else {
                continue;
            };

            let parameters = parameters.snapshot().typed().clone();
            let objects = objects.unwrap_or_default();
            let camera_matrix = camera_matrix.inner.clone();
            let robot_to_camera = robot_to_camera(&camera_matrix);
            let field_dimensions = *field_dimensions.as_ref();
            let pose_hint = localization_cache
                .get_nearest_with_stamp(image_time)
                .and_then(|(stamp, localization)| {
                    (time_distance(stamp, image_time) <= parameters.pose_hint.max_pose_age)
                        .then(|| {
                            localization
                                .inner
                                .as_ref()
                                .map(|pose| pose.clone().inverse())
                        })
                        .flatten()
                });
            let include_debug = global_localization_publisher.has_subscribers();

            let mut state = std::mem::take(&mut association_state);
            let (state, localization) = tokio::task::spawn_blocking(move || {
                let visual_features = find_detected_visual_features(&objects);
                if visual_features.supported_feature_count() == 0 {
                    return (
                        state,
                        GlobalVisualLocalization {
                            debug: None,
                            accepted_global_pose: None,
                            associations: Vec::new(),
                        },
                    );
                }

                let localization = state.associate_visual_features_with_debug(
                    &visual_features,
                    &camera_matrix,
                    &field_dimensions,
                    pose_hint,
                    &parameters,
                    include_debug,
                );
                (state, localization)
            })
            .await
            .wrap_err("field mark association task failed")?;
            association_state = state;

            let debug = localization.debug.clone();
            global_localization_publisher
                .publish_if_subscribed(|| ready(debug))
                .await?;
            if localization.accepted_global_pose.is_some() {
                global_pose_publisher
                    .publish(&TimeWrapper {
                        time: image_time,
                        inner: localization.accepted_global_pose,
                    })
                    .await?;
            }

            let message = TimeWrapper {
                time: image_time,
                inner: FieldMarkAssociations {
                    robot_to_camera,
                    associations: localization.associations,
                },
            };
            associations_publisher.publish(&message).await?;
        }
    }
}

fn camera_matrix_is_fresh(
    camera_matrix: &TimeWrapper<CameraMatrix>,
    time: ros_z::time::Time,
) -> bool {
    time_distance(camera_matrix.time, time) <= MAX_CAMERA_MATRIX_TIME_DISTANCE
}

fn time_distance(a: ros_z::time::Time, b: ros_z::time::Time) -> Duration {
    Duration::from_nanos(a.as_nanos().abs_diff(b.as_nanos()))
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

impl FieldMarkAssociationState {
    /// Associates visual field features while preserving pose-hint health and global recovery state.
    pub fn associate_visual_features_with_debug(
        &mut self,
        visual_features: &DetectedVisualFeatures,
        camera_matrix: &CameraMatrix,
        field_dimensions: &FieldDimensions,
        pose_hint: Option<Isometry3<Robot, Field>>,
        parameters: &FieldMarkAssociationParameters,
        include_debug: bool,
    ) -> GlobalVisualLocalization {
        let localizer = GlobalAssociator::new(parameters.global_localizer);
        let input = GlobalLocalizationInput {
            visual_features,
            field_dimensions,
            ground_to_robot: camera_matrix.ground_to_robot,
            robot_to_camera: robot_to_camera(camera_matrix),
            camera_intrinsic: camera_matrix.intrinsics,
            pose_hint,
        };

        let pose_hint_result =
            localizer.associate_with_pose_hint(input.clone(), parameters.pose_hint);
        if self.has_global_lock && pose_hint_result.is_healthy(parameters.pose_hint) {
            self.reset_recovery();
            return GlobalVisualLocalization {
                debug: None,
                accepted_global_pose: None,
                associations: pose_hint_field_mark_associations(&pose_hint_result),
            };
        }

        let trusted_pose_hint = if self.has_global_lock {
            pose_hint
        } else {
            None
        };
        let result = localizer.localize(GlobalLocalizationInput {
            pose_hint: trusted_pose_hint,
            ..input
        });
        let debug = if include_debug {
            result.as_ref().map(global_localization_debug_from_result)
        } else {
            None
        };
        let accepted =
            self.global_recovery_associations(result.as_ref(), trusted_pose_hint, parameters);

        GlobalVisualLocalization {
            debug,
            accepted_global_pose: accepted.global_pose,
            associations: accepted.associations,
        }
    }

    fn reset_recovery(&mut self) {
        self.pending_global_recovery = None;
    }

    fn global_recovery_associations(
        &mut self,
        result: Option<&GlobalLocalizationResult>,
        pose_hint: Option<Isometry3<Robot, Field>>,
        parameters: &FieldMarkAssociationParameters,
    ) -> AcceptedFieldMarkAssociations {
        let Some(result) = result else {
            self.pending_global_recovery = None;
            return AcceptedFieldMarkAssociations::empty();
        };
        let Some(associations) = result.unique_feature_associations() else {
            self.pending_global_recovery = None;
            return AcceptedFieldMarkAssociations::empty();
        };
        let robot_to_field = result.associations().robot_to_field;
        let associations =
            field_mark_associations(associations.iter(), FieldMarkAssociationKind::GlobalUnique);
        if let Some(pose_hint) = pose_hint
            && poses_agree(robot_to_field, pose_hint, parameters.pose_hint)
        {
            return self.accept_global_associations(robot_to_field, associations, false);
        }

        if self.has_global_lock {
            let Some(branch_hint) = pose_hint else {
                self.pending_global_recovery = None;
                return AcceptedFieldMarkAssociations::empty();
            };
            if !poses_are_on_same_symmetry_branch(robot_to_field, branch_hint, parameters.pose_hint)
            {
                self.pending_global_recovery = None;
                return AcceptedFieldMarkAssociations::empty();
            }

            return self.stage_global_recovery(robot_to_field, associations, parameters);
        }

        self.stage_global_recovery(robot_to_field, associations, parameters)
    }

    fn stage_global_recovery(
        &mut self,
        robot_to_field: Isometry3<Robot, Field>,
        associations: Vec<FieldMarkAssociation>,
        parameters: &FieldMarkAssociationParameters,
    ) -> AcceptedFieldMarkAssociations {
        let consecutive_frames = self
            .pending_global_recovery
            .as_ref()
            .filter(|pending| {
                poses_agree(robot_to_field, pending.robot_to_field, parameters.pose_hint)
            })
            .map_or(1, |pending| pending.consecutive_frames + 1);
        self.pending_global_recovery = Some(GlobalRecoveryCandidate {
            robot_to_field,
            associations: associations.clone(),
            consecutive_frames,
        });
        if consecutive_frames >= parameters.pose_hint.recovery_frames {
            let Some(pending) = self.pending_global_recovery.take() else {
                return AcceptedFieldMarkAssociations::empty();
            };
            self.accept_global_associations(pending.robot_to_field, pending.associations, true)
        } else {
            AcceptedFieldMarkAssociations::empty()
        }
    }

    fn accept_global_associations(
        &mut self,
        robot_to_field: Isometry3<Robot, Field>,
        associations: Vec<FieldMarkAssociation>,
        reset_backend: bool,
    ) -> AcceptedFieldMarkAssociations {
        self.has_global_lock = true;
        self.pending_global_recovery = None;
        if reset_backend {
            AcceptedFieldMarkAssociations::global(robot_to_field, associations)
        } else {
            AcceptedFieldMarkAssociations::associations(associations)
        }
    }
}

fn pose_hint_field_mark_associations(
    result: &PoseHintAssociationResult,
) -> Vec<FieldMarkAssociation> {
    field_mark_associations(
        result.associations.iter(),
        FieldMarkAssociationKind::PoseHint,
    )
}

fn poses_agree(
    left: Isometry3<Robot, Field>,
    right: Isometry3<Robot, Field>,
    config: PoseHintAssociationParameters,
) -> bool {
    let translation_delta = left.inner.translation.vector - right.inner.translation.vector;
    let translation_distance =
        nalgebra::Vector2::new(translation_delta.x, translation_delta.y).norm();
    let yaw_distance = yaw_difference(left, right).abs();
    translation_distance <= config.recovery_max_pose_distance
        && yaw_distance <= config.recovery_max_pose_angle
}

fn poses_are_on_same_symmetry_branch(
    left: Isometry3<Robot, Field>,
    right: Isometry3<Robot, Field>,
    config: PoseHintAssociationParameters,
) -> bool {
    yaw_difference(left, right).abs() <= config.recovery_max_branch_yaw_error
}

fn yaw_difference(left: Isometry3<Robot, Field>, right: Isometry3<Robot, Field>) -> f32 {
    let (_, _, left_yaw) = left.inner.rotation.euler_angles();
    let (_, _, right_yaw) = right.inner.rotation.euler_angles();
    let mut difference = left_yaw - right_yaw;
    while difference > std::f32::consts::PI {
        difference -= std::f32::consts::TAU;
    }
    while difference < -std::f32::consts::PI {
        difference += std::f32::consts::TAU;
    }
    difference
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
            .and_then(GlobalLocalizationResult::unique_feature_associations)
            .map(|associations| {
                field_mark_associations(associations.iter(), FieldMarkAssociationKind::GlobalUnique)
            })
            .unwrap_or_default(),
    }
}

fn field_mark_associations<'a>(
    associations: impl IntoIterator<Item = &'a FeatureAssociation>,
    kind: FieldMarkAssociationKind,
) -> Vec<FieldMarkAssociation> {
    associations
        .into_iter()
        .map(|association| FieldMarkAssociation {
            detection: association.detection,
            field_point: association.field_point.extend(0.0),
            kind,
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

fn global_localization_debug_from_result(
    result: &GlobalLocalizationResult,
) -> GlobalLocalizationDebug {
    let status = match result {
        GlobalLocalizationResult::Ambiguous(_) => GlobalLocalizationDebugStatus::Ambiguous,
        GlobalLocalizationResult::UniqueModuloSymmetry(_) => {
            GlobalLocalizationDebugStatus::UniqueModuloSymmetry
        }
    };
    let associations = result.associations();
    GlobalLocalizationDebug {
        robot_to_field: associations.robot_to_field,
        status,
        inliers: associations.score.inliers,
        candidate_score: associations.score.candidate_score,
        metric_rms_residual: associations.score.metric_rms_residual,
        reprojection_rmse: associations.score.reprojection_rmse,
        total_cost: associations.score.total_cost,
    }
}

/// Extracts goalpost image points from object detections.
pub fn find_detected_goalposts(detections: &[Object<RobocupObjectLabel>]) -> Vec<Point2<Pixel>> {
    find_detected_visual_features(detections)
        .goalposts
        .into_iter()
        .map(|feature| feature.pixel)
        .collect()
}

/// Field-feature detection used by global localization.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DetectedVisualFeature {
    /// Image point used for projection and association.
    pub pixel: Point2<Pixel>,
    /// Detector confidence in `[0, 1]`; invalid or low-confidence detections are ignored later.
    pub confidence: f32,
}

impl DetectedVisualFeature {
    fn new(pixel: Point2<Pixel>, confidence: f32) -> Self {
        Self { pixel, confidence }
    }
}

/// Field-feature detections grouped by the landmark class used by global localization.
#[derive(Debug, Default, PartialEq)]
pub struct DetectedVisualFeatures {
    /// Goalpost detections, represented by bottom-center image points.
    pub goalposts: Vec<DetectedVisualFeature>,
    /// L-crossing spot detections, represented by bounding-box centers.
    pub l_spots: Vec<DetectedVisualFeature>,
    /// T-crossing spot detections, represented by bounding-box centers.
    pub t_spots: Vec<DetectedVisualFeature>,
    /// Penalty spot detections, represented by bounding-box centers.
    pub penalty_spots: Vec<DetectedVisualFeature>,
    /// x-spot detections, represented by bounding-box centers.
    pub x_spots: Vec<DetectedVisualFeature>,
}

impl DetectedVisualFeatures {
    /// Counts detections from classes supported by global localization.
    pub fn supported_feature_count(&self) -> usize {
        self.goalposts.len()
            + self.l_spots.len()
            + self.t_spots.len()
            + self.penalty_spots.len()
            + self.x_spots.len()
    }
}

/// Extracts all field-feature detections supported by global localization.
pub fn find_detected_visual_features(
    detections: &[Object<RobocupObjectLabel>],
) -> DetectedVisualFeatures {
    detections
        .iter()
        .fold(DetectedVisualFeatures::default(), |mut features, object| {
            let confidence = object.bounding_box.confidence;
            match object.label {
                RobocupObjectLabel::GoalPost => features.goalposts.push(
                    DetectedVisualFeature::new(pixel_bottom_center(object), confidence),
                ),
                RobocupObjectLabel::LSpot => features
                    .l_spots
                    .push(DetectedVisualFeature::new(pixel_center(object), confidence)),
                RobocupObjectLabel::TSpot => features
                    .t_spots
                    .push(DetectedVisualFeature::new(pixel_center(object), confidence)),
                RobocupObjectLabel::PenaltySpot => features
                    .penalty_spots
                    .push(DetectedVisualFeature::new(pixel_center(object), confidence)),
                RobocupObjectLabel::XSpot => features
                    .x_spots
                    .push(DetectedVisualFeature::new(pixel_center(object), confidence)),
                _ => {}
            }
            features
        })
}

fn pixel_bottom_center(object: &Object<RobocupObjectLabel>) -> Point2<Pixel> {
    let area = object.bounding_box.area;
    point![(area.min.x() + area.max.x()) * 0.5, area.max.y()]
}

fn pixel_center(object: &Object<RobocupObjectLabel>) -> Point2<Pixel> {
    let area = object.bounding_box.area;
    point![
        (area.min.x() + area.max.x()) * 0.5,
        (area.min.y() + area.max.y()) * 0.5
    ]
}

fn robot_to_camera(camera_matrix: &CameraMatrix) -> Isometry3<Robot, Camera> {
    camera_matrix.head_to_camera * camera_matrix.robot_to_head
}

#[cfg(test)]
mod tests {
    use geometry::rectangle::Rectangle;
    use types::bounding_box::BoundingBox;

    use super::global_association::{FeatureAssociations, GlobalLocalizationScore};
    use super::*;

    #[test]
    fn goalpost_detection_uses_pixel_bottom_center() {
        let detections = vec![Object {
            label: RobocupObjectLabel::GoalPost,
            bounding_box: BoundingBox {
                area: Rectangle {
                    min: point![10.0, 20.0],
                    max: point![30.0, 50.0],
                },
                confidence: 1.0,
            },
        }];

        let goalposts = find_detected_goalposts(&detections);

        assert_eq!(goalposts.len(), 1);
        assert_eq!(goalposts[0], point![20.0, 50.0]);
    }

    #[test]
    fn spot_detections_use_pixel_center() {
        let detections = vec![
            Object {
                label: RobocupObjectLabel::LSpot,
                bounding_box: BoundingBox {
                    area: Rectangle {
                        min: point![10.0, 20.0],
                        max: point![30.0, 50.0],
                    },
                    confidence: 1.0,
                },
            },
            Object {
                label: RobocupObjectLabel::TSpot,
                bounding_box: BoundingBox {
                    area: Rectangle {
                        min: point![40.0, 60.0],
                        max: point![60.0, 80.0],
                    },
                    confidence: 1.0,
                },
            },
            Object {
                label: RobocupObjectLabel::PenaltySpot,
                bounding_box: BoundingBox {
                    area: Rectangle {
                        min: point![70.0, 90.0],
                        max: point![90.0, 110.0],
                    },
                    confidence: 1.0,
                },
            },
        ];

        let features = find_detected_visual_features(&detections);

        assert_eq!(feature_pixels(&features.l_spots), vec![point![20.0, 35.0]]);
        assert_eq!(feature_pixels(&features.t_spots), vec![point![50.0, 70.0]]);
        assert_eq!(
            feature_pixels(&features.penalty_spots),
            vec![point![80.0, 100.0]]
        );
        assert_eq!(
            features.l_spots.first().map(|feature| feature.confidence),
            Some(1.0)
        );
    }

    #[test]
    fn unhealthy_tracking_recovers_on_same_symmetry_branch_after_stable_frames() {
        let parameters = recovery_parameters();
        let global_pose = robot_to_field(2.0, 1.0, 0.2);
        let drifted_hint_on_same_branch = robot_to_field(-1.5, -1.0, 0.25);
        let result = unique_result(global_pose);
        let mut state = globally_locked_state();

        let first = state.global_recovery_associations(
            Some(&result),
            Some(drifted_hint_on_same_branch),
            &parameters,
        );
        let second = state.global_recovery_associations(
            Some(&result),
            Some(drifted_hint_on_same_branch),
            &parameters,
        );

        assert!(first.global_pose.is_none());
        assert!(first.associations.is_empty());
        let recovered_pose = second
            .global_pose
            .expect("stable same-branch recovery should reset backend");
        assert_eq!(
            recovered_pose.inner.translation.vector.x,
            global_pose.inner.translation.vector.x
        );
        assert_eq!(second.associations.len(), 1);
    }

    #[test]
    fn unhealthy_tracking_rejects_symmetric_branch_flip() {
        let parameters = recovery_parameters();
        let flipped_global_pose = robot_to_field(2.0, 1.0, std::f32::consts::PI + 0.2);
        let drifted_hint = robot_to_field(-1.5, -1.0, 0.2);
        let result = unique_result(flipped_global_pose);
        let mut state = globally_locked_state();

        let first =
            state.global_recovery_associations(Some(&result), Some(drifted_hint), &parameters);
        let second =
            state.global_recovery_associations(Some(&result), Some(drifted_hint), &parameters);

        assert!(first.global_pose.is_none());
        assert!(first.associations.is_empty());
        assert!(second.global_pose.is_none());
        assert!(second.associations.is_empty());
    }

    #[test]
    fn locked_recovery_without_pose_hint_fails_closed() {
        let parameters = recovery_parameters();
        let result = unique_result(robot_to_field(2.0, 1.0, 0.2));
        let mut state = globally_locked_state();

        let first = state.global_recovery_associations(Some(&result), None, &parameters);
        let second = state.global_recovery_associations(Some(&result), None, &parameters);

        assert!(first.global_pose.is_none());
        assert!(first.associations.is_empty());
        assert!(second.global_pose.is_none());
        assert!(second.associations.is_empty());
    }

    fn feature_pixels(features: &[DetectedVisualFeature]) -> Vec<Point2<Pixel>> {
        features.iter().map(|feature| feature.pixel).collect()
    }

    fn recovery_parameters() -> FieldMarkAssociationParameters {
        FieldMarkAssociationParameters {
            pose_hint: PoseHintAssociationParameters {
                recovery_frames: 2,
                ..Default::default()
            },
            ..Default::default()
        }
    }

    fn globally_locked_state() -> FieldMarkAssociationState {
        FieldMarkAssociationState {
            has_global_lock: true,
            ..Default::default()
        }
    }

    fn unique_result(robot_to_field: Isometry3<Robot, Field>) -> GlobalLocalizationResult {
        GlobalLocalizationResult::UniqueModuloSymmetry(FeatureAssociations {
            robot_to_field,
            features: vec![FeatureAssociation {
                detection_id: 0,
                landmark_id: 0,
                detection: point![<Pixel>, 1.0, 2.0],
                field_point: point![<Field>, 3.0, 4.0],
            }],
            score: GlobalLocalizationScore {
                inliers: 1,
                candidate_score: 1.0,
                metric_rms_residual: 0.0,
                reprojection_rmse: 0.0,
                total_cost: 0.0,
            },
        })
    }

    fn robot_to_field(x: f32, y: f32, yaw: f32) -> Isometry3<Robot, Field> {
        Isometry3::wrap(nalgebra::Isometry3::from_parts(
            nalgebra::Translation3::new(x, y, 0.0),
            nalgebra::UnitQuaternion::from_euler_angles(0.0, 0.0, yaw),
        ))
    }
}
