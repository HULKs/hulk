use std::time::Duration;

use nalgebra::{Matrix3, vector};
use serde::{Deserialize, Serialize};

use coordinate_systems::{Field, Ground};
use geometry::line_segment::LineSegment;
use linear_algebra::{Isometry2, Point2, Pose2};
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
    pub hypotheses: Vec<LocalizationDebugHypothesis>,
}

#[derive(Clone, Debug, Deserialize, Serialize, PathSerialize, PathDeserialize, PathIntrospect)]
pub struct LocalizationDebugHypothesis {
    pub ground_to_field: Isometry2<Ground, Field>,
    pub score: f32,
    pub covariance: Matrix3<f32>,
    pub line_associations: Vec<LineAssociationDebug>,
    pub unmatched_lines_in_field: Vec<LineSegment<Field>>,
    pub penalty_spot_associations: Vec<PointAssociationDebug>,
    pub unmatched_penalty_spots_in_field: Vec<Point2<Field>>,
    pub single_goal_post_associations: Vec<PointAssociationDebug>,
    pub unmatched_goal_posts_in_field: Vec<Point2<Field>>,
    pub goal_post_pair_association: Option<GoalPostPairAssociationDebug>,
}

#[derive(Clone, Debug, Deserialize, Serialize, PathSerialize, PathDeserialize, PathIntrospect)]
pub struct LineAssociationDebug {
    pub measured_line: LineSegment<Field>,
    pub matched_field_mark: FieldMark,
    pub correspondence_points: (CorrespondencePoints, CorrespondencePoints),
    pub fit_error: f32,
}

#[derive(Clone, Debug, Deserialize, Serialize, PathSerialize, PathDeserialize, PathIntrospect)]
pub struct PointAssociationDebug {
    pub measured_point_in_field: Point2<Field>,
    pub matched_reference_point: Option<Point2<Field>>,
    pub association_distance: Option<f32>,
    pub matching_distance: f32,
    pub accepted: bool,
}

#[derive(Clone, Debug, Deserialize, Serialize, PathSerialize, PathDeserialize, PathIntrospect)]
pub struct GoalPostPairAssociationDebug {
    pub measured_posts_in_field: (Point2<Field>, Point2<Field>),
    pub matched_reference_posts: (Point2<Field>, Point2<Field>),
    pub pair_fit_error: f32,
    pub matching_distance: f32,
    pub accepted: bool,
    pub resulting_ground_to_field: Option<Isometry2<Ground, Field>>,
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
