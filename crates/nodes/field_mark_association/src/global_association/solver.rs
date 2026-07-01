use std::{
    collections::{HashMap, HashSet},
    sync::{Arc, Mutex, OnceLock},
};

use ::types::field_dimensions::FieldDimensions;
use coordinate_systems::{Camera, Field, Ground, Pixel, Robot};
use hungarian_algorithm::AssignmentProblem;
use linear_algebra::{Isometry3, Point2, point};
use nalgebra::{Similarity2, Translation3, Vector2};
use ndarray::Array2;
use ordered_float::NotNan;
use projection::intrinsic::Intrinsic;

use crate::{DetectedVisualFeature, DetectedVisualFeatures};

use super::{
    FEATURE_CLASSES, FeatureAssociation, FeatureAssociations, GLOBAL_LOCALIZER_MAX_DETECTIONS,
    GlobalAssociationConfig, GlobalLocalizationDebugAssociation, GlobalLocalizationDebugDetection,
    GlobalLocalizationDebugProjection, GlobalLocalizationDetailedDebug,
    GlobalLocalizationDetailedStatus, GlobalLocalizationScore, PoseHintAssociationConfig,
    PoseHintAssociationResult, VisualFeatureClass,
    map::{LandmarkMap, MapTriplet, MapTripletBin, triplet_bin},
};

mod bounds;
mod certification;
mod cheap;
mod fitting;
mod output;
mod problem;
mod search;
mod seeding;
#[cfg(test)]
mod tests;
mod types;

use bounds::*;
use certification::*;
use cheap::*;
use fitting::*;
use output::*;
use problem::*;
use search::*;
use seeding::*;
use types::*;
pub(crate) use types::{GlobalLocalizationInput, GlobalLocalizationResult};

pub(crate) fn solve(
    input: GlobalLocalizationInput<'_>,
    config: GlobalAssociationConfig,
) -> Option<GlobalLocalizationResult> {
    let problem = Problem::new(input, config)?;
    solve_problem(&problem)
}

pub(crate) fn solve_detailed(
    input: GlobalLocalizationInput<'_>,
    config: GlobalAssociationConfig,
) -> Option<GlobalLocalizationDetailedDebug> {
    let problem = Problem::new(input, config)?;
    let result = solve_problem(&problem)?;
    Some(detailed_debug_from_result(&result, &problem))
}

pub(crate) fn associate_with_pose_hint(
    input: GlobalLocalizationInput<'_>,
    global_config: GlobalAssociationConfig,
    pose_config: PoseHintAssociationConfig,
) -> PoseHintAssociationResult {
    if !pose_config.enabled || !valid_intrinsic(input.camera_intrinsic) {
        return PoseHintAssociationResult::default();
    }
    let Some(robot_to_field) = input.pose_hint else {
        return PoseHintAssociationResult::default();
    };

    let map = cached_landmark_map(input.field_dimensions, global_config.min_map_baseline);
    let ground_to_camera = input.robot_to_camera * input.ground_to_robot;
    let camera_to_ground = ground_to_camera.inverse();
    let camera_origin = camera_to_ground.inner.translation.vector;
    if !camera_origin.iter().all(|value| value.is_finite())
        || camera_origin.z.abs() <= HORIZON_EPSILON
    {
        return PoseHintAssociationResult::default();
    }

    let detection_set = detection_points(
        input.visual_features,
        &map,
        &camera_to_ground,
        input.camera_intrinsic,
        global_config,
    );
    let field_to_camera =
        field_to_camera_from_robot_to_field(input.robot_to_camera, robot_to_field);

    let mut associations = Vec::new();
    let mut residuals = Vec::new();
    for class in FEATURE_CLASSES {
        let class_associations = pose_hint_assignments_for_class(
            &map,
            &detection_set.detections,
            field_to_camera,
            input.camera_intrinsic,
            pose_config,
            class,
        );
        for option in class_associations {
            let Some(detection) = detection_set.detections.get(option.detection_index) else {
                continue;
            };
            let Some(landmark) = map.landmarks.get(option.landmark_id) else {
                continue;
            };
            residuals.push(option.reprojection_error_px);
            associations.push(FeatureAssociation {
                detection_id: detection.id,
                landmark_id: landmark.id,
                detection: detection.pixel,
                field_point: landmark.xy,
            });
        }
    }

    let reprojection_rmse = reprojection_rmse(&residuals);
    PoseHintAssociationResult {
        associations,
        reprojection_rmse,
    }
}

#[derive(Clone, Copy)]
struct PoseHintOption {
    detection_index: usize,
    landmark_id: usize,
    confidence: f32,
    reprojection_error_px: f32,
}

fn pose_hint_assignments_for_class(
    map: &LandmarkMap,
    detections: &[DetectionPoint],
    field_to_camera: Isometry3<Field, Camera>,
    intrinsic: Intrinsic,
    config: PoseHintAssociationConfig,
    class: VisualFeatureClass,
) -> Vec<PoseHintOption> {
    let landmark_ids = map.landmarks_for_class(class);
    if landmark_ids.is_empty() {
        return Vec::new();
    }

    let mut row_ranges = Vec::new();
    let mut row_second_best_errors = Vec::new();
    let mut options = Vec::new();
    for (detection_index, detection) in detections
        .iter()
        .enumerate()
        .filter(|(_, detection)| detection.class == class)
    {
        let detection_options = pose_hint_options_for_detection(
            map,
            detection_index,
            detection,
            field_to_camera,
            intrinsic,
        );
        let Some(best) = detection_options.first() else {
            continue;
        };
        if best.reprojection_error_px > config.max_reprojection_error_px {
            continue;
        }

        let row_start = options.len();
        row_second_best_errors.push(
            detection_options
                .get(1)
                .map(|option| option.reprojection_error_px),
        );
        options.extend(
            detection_options.into_iter().take_while(|option| {
                option.reprojection_error_px <= config.max_reprojection_error_px
            }),
        );
        row_ranges.push(row_start..options.len());
    }
    if row_ranges.is_empty() {
        return Vec::new();
    }
    if row_ranges.len() == 1
        && let Some(second_best_error) = row_second_best_errors[0]
    {
        let best_error = options[row_ranges[0].start].reprojection_error_px;
        if second_best_error - best_error < config.second_best_reprojection_margin_px {
            return Vec::new();
        }
    }

    // Prefer a complete in-gate group over a slid subset of near-zero matches.
    let missing_assignment_penalty =
        config.max_reprojection_error_px.powi(2) * row_ranges.len().max(1) as f32 + 1.0;
    let Ok(missing_assignment) = NotNan::new(-missing_assignment_penalty) else {
        return Vec::new();
    };
    let mut costs = Array2::from_elem((row_ranges.len(), landmark_ids.len()), missing_assignment);
    for (row, range) in row_ranges.iter().enumerate() {
        for option in &options[range.clone()] {
            let Some(column) = landmark_ids
                .iter()
                .position(|landmark_id| *landmark_id == option.landmark_id)
            else {
                continue;
            };
            let value = pose_hint_assignment_value(config, option);
            let Ok(cost) = NotNan::new(value) else {
                continue;
            };
            costs[(row, column)] = cost;
        }
    }

    AssignmentProblem::from_costs(costs)
        .solve()
        .into_iter()
        .enumerate()
        .filter_map(|(row, assignment)| {
            let assignment = assignment?;
            if assignment.cost <= 0.0 {
                return None;
            }
            options[row_ranges[row].clone()]
                .iter()
                .find(|option| option.landmark_id == landmark_ids[assignment.to])
                .copied()
        })
        .collect()
}

fn pose_hint_options_for_detection(
    map: &LandmarkMap,
    detection_index: usize,
    detection: &DetectionPoint,
    field_to_camera: Isometry3<Field, Camera>,
    intrinsic: Intrinsic,
) -> Vec<PoseHintOption> {
    let mut candidates = map
        .landmarks_for_class(detection.class)
        .iter()
        .filter_map(|&landmark_id| {
            let landmark = map.landmarks.get(landmark_id)?;
            let projected = project_field_point(field_to_camera, intrinsic, landmark.xy)?;
            let reprojection_error_px = (projected - detection.pixel).inner.norm();
            reprojection_error_px
                .is_finite()
                .then_some((landmark_id, reprojection_error_px))
        })
        .collect::<Vec<_>>();
    candidates.sort_by(|left, right| left.1.total_cmp(&right.1));

    candidates
        .into_iter()
        .map(|(landmark_id, reprojection_error_px)| PoseHintOption {
            detection_index,
            landmark_id,
            confidence: detection.confidence,
            reprojection_error_px,
        })
        .collect()
}

fn pose_hint_assignment_value(config: PoseHintAssociationConfig, option: &PoseHintOption) -> f32 {
    let max_error = config.max_reprojection_error_px;
    (max_error.powi(2) - option.reprojection_error_px.powi(2)).max(0.0) + option.confidence * 1.0e-3
}

fn reprojection_rmse(residuals: &[f32]) -> Option<f32> {
    (!residuals.is_empty()).then(|| {
        (residuals
            .iter()
            .map(|residual| residual.powi(2))
            .sum::<f32>()
            / residuals.len() as f32)
            .sqrt()
    })
}
