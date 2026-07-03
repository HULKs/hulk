use coordinate_systems::{Field, Robot};
use linear_algebra::Isometry3;
use projection::camera_matrix::CameraMatrix;
use types::{
    field_dimensions::FieldDimensions,
    visual_localization::{FieldMarkAssociation, FieldMarkAssociationSource},
};

use crate::{
    api::{GlobalVisualLocalization, field_mark_associations},
    debug::global_localization_debug_from_result,
    features::DetectedVisualFeatures,
    global_association::{
        GlobalAssociator, GlobalLocalizationInput, GlobalLocalizationResult,
        PoseHintAssociationConfig as PoseHintAssociationParameters, PoseHintAssociationResult,
    },
    parameters::FieldMarkAssociationParameters,
    robot_to_camera,
};

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

impl FieldMarkAssociationState {
    pub(crate) fn reset_for_damping(&mut self) {
        self.pending_global_recovery = None;
        self.has_global_lock = false;
    }

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
        let input =
            global_localization_input(visual_features, camera_matrix, field_dimensions, pose_hint);

        if let Some(associations) =
            self.try_pose_hint_associations(&localizer, input.clone(), parameters.pose_hint)
        {
            return GlobalVisualLocalization {
                debug: None,
                accepted_global_pose: None,
                associations,
            };
        }

        let result = localizer.localize(input);
        let debug = global_debug_from_result(result.as_ref(), include_debug);
        let accepted = self.apply_global_recovery(
            result.as_ref(),
            self.trusted_recovery_pose_hint(pose_hint),
            parameters,
        );

        GlobalVisualLocalization {
            debug,
            accepted_global_pose: accepted.global_pose,
            associations: accepted.associations,
        }
    }

    fn try_pose_hint_associations(
        &mut self,
        localizer: &GlobalAssociator,
        input: GlobalLocalizationInput<'_>,
        config: PoseHintAssociationParameters,
    ) -> Option<Vec<FieldMarkAssociation>> {
        let result = localizer.associate_with_pose_hint(input, config);
        if !self.has_global_lock || !result.is_healthy(config) {
            return None;
        }

        self.reset_recovery();
        Some(pose_hint_field_mark_associations(&result))
    }

    fn trusted_recovery_pose_hint(
        &self,
        pose_hint: Option<Isometry3<Robot, Field>>,
    ) -> Option<Isometry3<Robot, Field>> {
        self.has_global_lock.then_some(pose_hint).flatten()
    }

    fn reset_recovery(&mut self) {
        self.pending_global_recovery = None;
    }

    fn apply_global_recovery(
        &mut self,
        result: Option<&GlobalLocalizationResult>,
        pose_hint: Option<Isometry3<Robot, Field>>,
        parameters: &FieldMarkAssociationParameters,
    ) -> AcceptedFieldMarkAssociations {
        let Some(result) = result else {
            self.pending_global_recovery = None;
            return AcceptedFieldMarkAssociations::empty();
        };
        let associations = result.unique_feature_associations();
        let robot_to_field = result.associations().robot_to_field;
        let associations = field_mark_associations(
            associations.iter(),
            FieldMarkAssociationSource::GlobalUnique,
        );
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

fn global_localization_input<'a>(
    visual_features: &'a DetectedVisualFeatures,
    camera_matrix: &CameraMatrix,
    field_dimensions: &'a FieldDimensions,
    pose_hint: Option<Isometry3<Robot, Field>>,
) -> GlobalLocalizationInput<'a> {
    GlobalLocalizationInput {
        visual_features,
        field_dimensions,
        ground_to_robot: camera_matrix.ground_to_robot,
        robot_to_camera: robot_to_camera(camera_matrix),
        camera_intrinsic: camera_matrix.intrinsics,
        pose_hint,
    }
}

fn global_debug_from_result(
    result: Option<&GlobalLocalizationResult>,
    include_debug: bool,
) -> Option<types::visual_localization::GlobalLocalizationDebug> {
    if include_debug {
        result.map(global_localization_debug_from_result)
    } else {
        None
    }
}

fn pose_hint_field_mark_associations(
    result: &PoseHintAssociationResult,
) -> Vec<FieldMarkAssociation> {
    field_mark_associations(
        result.associations.iter(),
        FieldMarkAssociationSource::PoseHint,
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

#[cfg(test)]
mod tests {
    use coordinate_systems::Pixel;
    use linear_algebra::point;

    use super::*;
    use crate::{
        PoseHintAssociationParameters,
        global_association::{FeatureAssociation, FeatureAssociations, GlobalLocalizationScore},
    };

    #[test]
    fn unhealthy_tracking_recovers_on_same_symmetry_branch_after_stable_frames() {
        let parameters = recovery_parameters();
        let global_pose = robot_to_field(2.0, 1.0, 0.2);
        let drifted_hint_on_same_branch = robot_to_field(-1.5, -1.0, 0.25);
        let result = unique_result(global_pose);
        let mut state = globally_locked_state();

        let first = state.apply_global_recovery(
            Some(&result),
            Some(drifted_hint_on_same_branch),
            &parameters,
        );
        let second = state.apply_global_recovery(
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

        let first = state.apply_global_recovery(Some(&result), Some(drifted_hint), &parameters);
        let second = state.apply_global_recovery(Some(&result), Some(drifted_hint), &parameters);

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

        let first = state.apply_global_recovery(Some(&result), None, &parameters);
        let second = state.apply_global_recovery(Some(&result), None, &parameters);

        assert!(first.global_pose.is_none());
        assert!(first.associations.is_empty());
        assert!(second.global_pose.is_none());
        assert!(second.associations.is_empty());
    }

    #[test]
    fn damping_reset_clears_lock_and_pending_recovery() {
        let parameters = recovery_parameters();
        let result = unique_result(robot_to_field(2.0, 1.0, 0.2));
        let mut state = globally_locked_state();
        let _ = state.apply_global_recovery(
            Some(&result),
            Some(robot_to_field(-1.5, -1.0, 0.25)),
            &parameters,
        );

        state.reset_for_damping();

        assert!(!state.has_global_lock);
        assert!(state.pending_global_recovery.is_none());
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
