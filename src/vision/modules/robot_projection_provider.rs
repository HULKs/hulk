use crate::data_type::{
    camera_matrix::CameraMatrix, image_data::ImageData, joint_sensor_data::JointSensorData,
    robot_projection::RobotProjection,
};

#[derive(Default)]
pub struct RobotProjectionProvider;

impl RobotProjectionProvider {
    pub fn cycle(
        &mut self,
        image_data: &ImageData,
        camera_matrix: &CameraMatrix,
        joint_sensor_data: &JointSensorData,
    ) -> (RobotProjection,) {
        todo!();
    }
}
