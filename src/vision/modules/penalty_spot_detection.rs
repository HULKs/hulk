use crate::data_type::{
    ball_data::BallData, camera_matrix::CameraMatrix, field_color::FieldColor,
    field_dimensions::FieldDimensions, filtered_segments::FilteredSegments, image_data::ImageData,
    penalty_spot_data::PenaltySpotData,
};

#[derive(Default)]
pub struct PenaltySpotDetection;

impl PenaltySpotDetection {
    pub fn cycle(
        &mut self,
        image_data: &ImageData,
        field_dimensions: &FieldDimensions,
        camera_matrix: &CameraMatrix,
        filtered_segments: &FilteredSegments,
        ball_data: &BallData,
        field_color: &FieldColor,
    ) -> (PenaltySpotData,) {
        todo!();
    }
}
