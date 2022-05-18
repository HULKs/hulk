use crate::data_type::{
    camera_matrix::CameraMatrix, image_data::ImageData, joint_sensor_data::JointSensorData,
    robot_projection::RobotProjection,
};

pub struct RobotProjectionProvider;

impl RobotProjectionProvider {
    fn new(_context: NewContext) -> anyhow::Result<Self> {
        Ok(Self)
    }

    fn cycle(
        &mut self,
        image_data: &ImageData,
        camera_matrix: &CameraMatrix,
        joint_sensor_data: &JointSensorData,
    ) -> (RobotProjection,) {
        todo!();
    }
}
