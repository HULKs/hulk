use std::time::Duration;

use nalgebra::{Matrix3, Vector3, vector};
use serde::{Deserialize, Serialize};

use coordinate_systems::{Field, Ground};
use geometry::line_segment::LineSegment;
use linear_algebra::{Isometry2, Pose2};
use path_serde::{PathDeserialize, PathIntrospect, PathSerialize};

use crate::{
    field_marks::{CorrespondencePoints, FieldMark},
    multivariate_normal_distribution::MultivariateNormalDistribution,
};

#[derive(Clone, Debug, Deserialize, Serialize, PathSerialize, PathDeserialize, PathIntrospect)]
pub struct LocalizationDebugFrame {
    pub cycle_start_time: Duration,
    pub gyro_movement: f32,
    pub best_hypothesis_index: Option<usize>,
    pub measured_lines_in_field: Vec<LineSegment<Field>>,
    pub hypotheses: Vec<LocalizationDebugHypothesis>,
}

#[derive(Clone, Debug, Deserialize, Serialize, PathSerialize, PathDeserialize, PathIntrospect)]
pub struct LocalizationDebugHypothesis {
    pub ground_to_field: Isometry2<Ground, Field>,
    pub score: f32,
    pub covariance: Matrix3<f32>,
    pub correction_delta: Vector3<f32>,
    pub fit_error: f32,
    pub matched_lines: usize,
    pub unmatched_lines: usize,
    pub global_consensus: f32,
    pub local_ambiguity: f32,
    pub locality_confidence: f32,
    pub measurement_confidence: f32,
    pub observability_covariance: Matrix3<f32>,
    pub observability_variances: Vector3<f32>,
    pub final_matches: Vec<LocalizationMatchDebug>,
    pub candidate_summaries: Vec<MeasuredLineDebug>,
}

#[derive(Clone, Debug, Deserialize, Serialize, PathSerialize, PathDeserialize, PathIntrospect)]
pub struct LocalizationMatchDebug {
    pub measured_line_index: usize,
    pub measured_line: LineSegment<Field>,
    pub field_mark: FieldMark,
    pub correspondence_points: (CorrespondencePoints, CorrespondencePoints),
    pub geometric_cost: f32,
    pub locality_cost: f32,
    pub total_cost: f32,
    pub ambiguity_margin: f32,
}

#[derive(Clone, Debug, Deserialize, Serialize, PathSerialize, PathDeserialize, PathIntrospect)]
pub struct MeasuredLineDebug {
    pub measured_line_index: usize,
    pub measured_line: LineSegment<Field>,
    pub status: MeasuredLineStatus,
    pub selected_field_mark: Option<FieldMark>,
    pub best_geometric_cost: Option<f32>,
    pub best_locality_cost: Option<f32>,
    pub best_total_cost: Option<f32>,
    pub inlier_threshold: f32,
    pub rejection_reason: Option<MeasuredLineRejectionReason>,
    pub candidates: Vec<CandidateAlternativeDebug>,
}

#[derive(
    Clone,
    Copy,
    Debug,
    PartialEq,
    Eq,
    Deserialize,
    Serialize,
    PathSerialize,
    PathDeserialize,
    PathIntrospect,
)]
pub enum MeasuredLineStatus {
    Matched,
    RejectedByThreshold,
    NoCandidate,
}

#[derive(
    Clone,
    Copy,
    Debug,
    PartialEq,
    Eq,
    Deserialize,
    Serialize,
    PathSerialize,
    PathDeserialize,
    PathIntrospect,
)]
pub enum MeasuredLineRejectionReason {
    NoCandidate,
    TotalCostAboveThreshold,
}

#[derive(Clone, Debug, Deserialize, Serialize, PathSerialize, PathDeserialize, PathIntrospect)]
pub struct CandidateAlternativeDebug {
    pub field_mark: FieldMark,
    pub geometric_cost: f32,
    pub locality_cost: f32,
    pub total_cost: f32,
    pub accepted: bool,
}

#[derive(
    Clone, Copy, Debug, Serialize, Deserialize, PathSerialize, PathDeserialize, PathIntrospect,
)]
pub struct ScoredPose {
    pub state: MultivariateNormalDistribution<3>,
    pub score: f32,
}

impl ScoredPose {
    pub fn from_isometry(pose: Pose2<Field>, covariance: Matrix3<f32>, score: f32) -> Self {
        Self {
            state: MultivariateNormalDistribution {
                mean: vector![
                    pose.position().x(),
                    pose.position().y(),
                    pose.orientation().angle(),
                ],
                covariance,
            },
            score,
        }
    }
}
