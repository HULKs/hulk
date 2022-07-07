use nalgebra::Point2;
use serde::{Deserialize, Serialize};
use serialize_hierarchy::SerializeHierarchy;

use types::{
    Ball, CameraMatrix, CandidateEvaluation, ClusterCone, CycleInfo, DetectedRobots, FieldBorder,
    FieldColor, FilteredSegments, Image422, ImageLines, ImageSegments, LineData,
    PerspectiveGridCandidates, ScoredClusterPoint,
};

#[derive(Clone, Debug, Default, SerializeHierarchy)]
pub struct MainOutputs {
    pub balls: Option<Vec<Ball>>,
    pub camera_matrix: Option<CameraMatrix>,
    pub cycle_info: Option<CycleInfo>,
    pub detected_robots: Option<DetectedRobots>,
    pub field_border: Option<FieldBorder>,
    pub field_color: Option<FieldColor>,
    pub filtered_segments: Option<FilteredSegments>,
    pub image_segments: Option<ImageSegments>,
    pub line_data: Option<LineData>,
    pub perspective_grid_candidates: Option<PerspectiveGridCandidates>,
}

#[derive(Debug, Default, Clone, SerializeHierarchy)]
pub struct AdditionalOutputs {
    pub ball_candidates: Option<Vec<CandidateEvaluation>>,
    pub lines_in_image: Option<ImageLines>,
    pub field_border_points: Option<Vec<Point2<f32>>>,
    pub robot_detection: RobotDetection,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize, SerializeHierarchy)]
pub struct RobotDetection {
    pub cluster_points_in_pixel: Option<Vec<ScoredClusterPoint>>,
    pub clustered_cluster_points_in_ground: Option<Vec<Vec<ScoredClusterPoint>>>,
    pub cluster_cones: Option<Vec<ClusterCone>>,
}

#[derive(Debug, Default, Clone)]
pub struct Database {
    pub main_outputs: MainOutputs,
    pub additional_outputs: AdditionalOutputs,
    pub image: Option<Image422>,
}
