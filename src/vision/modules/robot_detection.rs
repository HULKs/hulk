use crate::data_type::{
    body_pose::BodyPose, camera_matrix::CameraMatrix, field_color::FieldColor,
    field_dimensions::FieldDimensions, image_data::ImageData, image_segments::ImageSegments,
    robot_data::RobotData,
};

pub struct RobotDetection;

impl RobotDetection {
    fn new(_context: NewContext) -> anyhow::Result<Self> {
        Ok(Self)
    }

    fn cycle(
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
