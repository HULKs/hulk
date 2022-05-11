use crate::data_type::{
    body_pose::BodyPose, camera_matrix::CameraMatrix, field_color::FieldColor,
    field_dimensions::FieldDimensions, image_data::ImageData, image_segments::ImageSegments,
    robot_data::RobotData,
};

#[derive(Default)]
pub struct RobotDetection;

impl RobotDetection {
    pub fn cycle(
        &mut self,
        body_pose: &BodyPose,
        camera_matrix: &CameraMatrix,
        field_color: &FieldColor,
        field_dimensions: &FieldDimensions,
        image_data: &ImageData,
        image_segments: &ImageSegments,
    ) -> (RobotData,) {
        todo!();
    }
}
