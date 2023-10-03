use std::time::SystemTime;

use color_eyre::eyre::Result;
use types::camera_position::CameraPosition;
use types::hardware::Ids;
use types::{
    audio::SpeakerRequest,
    hardware::Paths,
    joints::Joints,
    led::Leds,
    messages::{IncomingMessage, OutgoingMessage},
    samples::Samples,
    sensor_data::SensorData,
    ycbcr422_image::YCbCr422Image,
};

pub trait ActuatorInterface {
    fn write_to_actuators(
        &self,
        positions: Joints<f32>,
        stiffnesses: Joints<f32>,
        leds: Leds,
    ) -> Result<()>;
}

pub trait CameraInterface {
    fn read_from_camera(&self, camera_position: CameraPosition) -> Result<YCbCr422Image>;
}

pub trait IdInterface {
    fn get_ids(&self) -> Ids;
}

pub trait MicrophoneInterface {
    fn read_from_microphones(&self) -> Result<Samples>;
}

pub trait NetworkInterface {
    fn read_from_network(&self) -> Result<IncomingMessage>;
    fn write_to_network(&self, message: OutgoingMessage) -> Result<()>;
}

pub trait PathsInterface {
    fn get_paths(&self) -> Paths;
}

pub trait RecordingInterface {
    fn should_record(&self) -> bool;
    fn set_whether_to_record(&self, enable: bool);
}

pub trait SensorInterface {
    fn read_from_sensors(&self) -> Result<SensorData>;
}

pub trait SpeakerInterface {
    fn write_to_speakers(&self, request: SpeakerRequest);
}

pub trait TimeInterface {
    fn get_now(&self) -> SystemTime;
}
