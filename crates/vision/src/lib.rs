pub mod ball_detection;
pub mod camera_matrix_extractor;
pub mod field_border_detection;
pub mod field_color_detection;
pub mod image_segmenter;
pub mod line_detection;
pub mod perspective_grid_candidates_provider;
pub mod robot_detection;
pub mod segment_filter;

pub enum CyclerInstance {
    VisionTop,
    VisionBottom,
}
