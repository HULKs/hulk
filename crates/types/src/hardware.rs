use std::{fmt::Debug, time::SystemTime};

use color_eyre::Result;

use crate::{
    messages::{IncomingMessage, OutgoingMessage},
    samples::Samples,
    ycbcr422_image::YCbCr422Image,
};

use super::{CameraPosition, Joints, Leds, SensorData};

pub trait Interface {
    fn read_from_microphones(&self) -> Result<Samples>;

    fn get_now(&self) -> SystemTime;
    fn get_ids(&self) -> Ids;
    fn read_from_sensors(&self) -> Result<SensorData>;
    fn write_to_actuators(&self, positions: Joints, stiffnesses: Joints, leds: Leds) -> Result<()>;

    fn read_from_network(&self) -> Result<IncomingMessage>;
    fn write_to_network(&self, message: OutgoingMessage) -> Result<()>;

    fn read_from_camera(&self, camera_position: CameraPosition) -> Result<YCbCr422Image>;
}

#[derive(Clone, Debug)]
pub struct Ids {
    pub body_id: String,
    pub head_id: String,
}
