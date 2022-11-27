use nao_camera::Parameters as CameraParameters;
use serde::Deserialize;

#[derive(Clone, Debug, Deserialize)]
pub struct Parameters {
    pub camera_top: CameraParameters,
    pub camera_bottom: CameraParameters,
}
