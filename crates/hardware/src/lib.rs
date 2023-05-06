use std::time::SystemTime;

use color_eyre::eyre::Result;
use types::{
    hardware::Ids,
    messages::{IncomingMessage, OutgoingMessage},
    samples::Samples,
    ycbcr422_image::YCbCr422Image,
    CameraPosition, Joints, Leds, SensorData,
};

pub trait TimeInterface {
    fn get_now(&self) -> SystemTime;
}

pub trait SensorInterface {
    fn read_from_sensors(&self) -> Result<SensorData>;
}

pub trait MicrophoneInterface {
    fn read_from_microphones(&self) -> Result<Samples>;
}

pub trait IdInterface {
    fn get_ids(&self) -> Ids;
}

pub trait ActuatorInterface {
    fn write_to_actuators(
        &self,
        positions: Joints<f32>,
        stiffnesses: Joints<f32>,
        leds: Leds,
    ) -> Result<()>;
}

pub trait NetworkInterface {
    fn read_from_network(&self) -> Result<IncomingMessage>;
    fn write_to_network(&self, message: OutgoingMessage) -> Result<()>;
}

pub trait CameraInterface {
    fn read_from_camera(&self, camera_position: CameraPosition) -> Result<YCbCr422Image>;
}
