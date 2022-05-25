use parking_lot::Mutex;

use crate::types::{CameraPosition, CycleInfo, Image422, Joints, Leds, SensorData};

#[derive(Clone, Debug)]
pub struct HardwareIds {
    pub body_id: String,
    pub head_id: String,
}

pub const AUDIO_SAMPLE_RATE: u32 = 44100;
pub const NUMBER_OF_AUDIO_CHANNELS: usize = 4;
pub const NUMBER_OF_AUDIO_SAMPLES: usize = 2048;

pub trait HardwareInterface {
    fn get_ids(&self) -> HardwareIds;
    fn set_leds(&self, leds: Leds);
    fn set_joint_positions(&self, requested_positions: Joints);
    fn set_joint_stiffnesses(&self, requested_stiffnesses: Joints);
    fn produce_sensor_data(&self) -> anyhow::Result<SensorData>;
    fn produce_image_data(&self, camera_position: CameraPosition) -> anyhow::Result<CycleInfo>;
    fn get_image(&self, camera_position: CameraPosition) -> &Mutex<Image422>;
    fn start_image_capture(&self, camera_position: CameraPosition) -> anyhow::Result<()>;
    fn produce_audio_data(&self) -> anyhow::Result<()>;
    fn get_audio_buffer(
        &self,
    ) -> &Mutex<[[f32; NUMBER_OF_AUDIO_SAMPLES]; NUMBER_OF_AUDIO_CHANNELS]>;
}
