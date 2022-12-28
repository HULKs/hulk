use std::{fmt::Debug, sync::Arc, time::SystemTime};

use color_eyre::Result;
use spl_network_messages::{GameControllerReturnMessage, GameControllerStateMessage, SplMessage};

use crate::image::Image;

use super::{CameraPosition, Joints, Leds, SensorData};

pub trait Interface {
    fn read_from_microphones(&self) -> Result<Samples>;

    fn get_now(&self) -> SystemTime;
    fn get_ids(&self) -> Ids;
    fn read_from_sensors(&self) -> Result<SensorData>;
    fn write_to_actuators(&self, positions: Joints, stiffnesses: Joints, leds: Leds) -> Result<()>;

    fn read_from_network(&self) -> Result<IncomingMessage>;
    fn write_to_network(&self, message: OutgoingMessage) -> Result<()>;

    fn read_from_camera(&self, camera_position: CameraPosition) -> Result<Image>;
}

#[derive(Clone, Debug)]
pub struct Ids {
    pub body_id: String,
    pub head_id: String,
}

#[derive(Clone, Debug)]
pub enum IncomingMessage {
    GameController(GameControllerStateMessage),
    Spl(SplMessage),
}

impl Default for IncomingMessage {
    fn default() -> Self {
        IncomingMessage::GameController(Default::default())
    }
}

#[derive(Clone, Debug)]
pub enum OutgoingMessage {
    GameController(GameControllerReturnMessage),
    Spl(SplMessage),
}

impl Default for OutgoingMessage {
    fn default() -> Self {
        OutgoingMessage::GameController(Default::default())
    }
}

#[derive(Clone, Debug, Default)]
pub struct Samples {
    pub rate: u32,
    pub channels_of_samples: Arc<Vec<Vec<f32>>>,
}
