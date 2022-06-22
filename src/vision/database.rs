use macros::SerializeHierarchy;
use nalgebra::Point2;

use crate::types::{
    Ball, CameraMatrix, CandidateEvaluation, CycleInfo, FieldBorder, FieldColor, FilteredSegments,
    Image422, ImageLines, ImageSegments, Limb, LineData, PerspectiveGridCandidates,
};

#[derive(Clone, Debug, Default, SerializeHierarchy)]
pub struct MainOutputs {
    pub balls: Option<Vec<Ball>>,
    pub camera_matrix: Option<CameraMatrix>,
    pub cycle_info: Option<CycleInfo>,
    pub field_border: Option<FieldBorder>,
    pub field_color: Option<FieldColor>,
    pub filtered_segments: Option<FilteredSegments>,
    pub image_segments: Option<ImageSegments>,
    pub line_data: Option<LineData>,
    pub perspective_grid_candidates: Option<PerspectiveGridCandidates>,
    pub projected_limbs: Option<Vec<Limb>>,
}

#[derive(Debug, Default, Clone, SerializeHierarchy)]
pub struct AdditionalOutputs {
    pub ball_candidates: Option<Vec<CandidateEvaluation>>,
    pub lines_in_image: Option<ImageLines>,
    pub field_border_points: Option<Vec<Point2<f32>>>,
}

#[derive(Debug, Default, Clone)]
pub struct Database {
    pub main_outputs: MainOutputs,
    pub additional_outputs: AdditionalOutputs,
    pub image: Option<Image422>,
}
