use nao_camera::Parameters as CameraParameters;
use serde::Deserialize;

use super::microphones::Parameters as MicrophoneParameters;

#[derive(Clone, Debug, Deserialize)]
pub struct Parameters {
    pub microphones: MicrophoneParameters,
    pub camera_top: CameraParameters,
    pub camera_bottom: CameraParameters,
}
