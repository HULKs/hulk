use nao_camera::Parameters as CameraParameters;
use serde::Deserialize;

use crate::network::Parameters as NetworkParameters;

use super::microphones::Parameters as MicrophoneParameters;

#[derive(Clone, Debug, Deserialize)]
pub struct Parameters {
    pub microphones: MicrophoneParameters,
    pub network: NetworkParameters,
    pub camera_top: CameraParameters,
    pub camera_bottom: CameraParameters,
}
