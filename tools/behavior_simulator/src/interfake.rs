use std::{
    mem::take,
    sync::{Arc, Mutex},
    time::SystemTime,
};

use color_eyre::Result;
use types::{
    hardware::{Ids, Interface},
    messages::{IncomingMessage, OutgoingMessage},
    nao_image::NaoImage,
    samples::Samples,
    CameraPosition, Joints, SensorData,
};

#[derive(Default)]
pub struct Interfake {
    messages: Arc<Mutex<Vec<OutgoingMessage>>>,
}

impl Interface for Interfake {
    fn read_from_microphones(&self) -> Result<Samples> {
        unimplemented!()
    }

    fn get_now(&self) -> SystemTime {
        SystemTime::now()
    }

    fn get_ids(&self) -> Ids {
        unimplemented!()
    }

    fn read_from_sensors(&self) -> Result<SensorData> {
        unimplemented!()
    }

    fn write_to_actuators(
        &self,
        _positions: Joints,
        _stiffnesses: Joints,
        _leds: types::Leds,
    ) -> Result<()> {
        unimplemented!()
    }

    fn read_from_network(&self) -> Result<IncomingMessage> {
        unimplemented!()
    }

    fn write_to_network(&self, message: OutgoingMessage) -> Result<()> {
        self.messages.lock().unwrap().push(message);
        Ok(())
    }

    fn read_from_camera(&self, _camera_position: CameraPosition) -> Result<NaoImage> {
        unimplemented!()
    }
}

impl Interfake {
    pub fn take_outgoing_messages(&self) -> Vec<OutgoingMessage> {
        take(&mut self.messages.lock().unwrap())
    }
}
