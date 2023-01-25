use std::{
    mem::take,
    sync::{Arc, Mutex},
    time::SystemTime,
};

use color_eyre::Result;
use types::{
    hardware::{Ids, Interface},
    image::Image,
    messages::{IncomingMessage, OutgoingMessage},
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
        positions: Joints,
        stiffnesses: Joints,
        leds: types::Leds,
    ) -> Result<()> {
        unimplemented!()
    }

    fn read_from_network(&self) -> Result<IncomingMessage> {
        unimplemented!()
    }

    fn write_to_network(&self, message: OutgoingMessage) -> Result<()> {
        println!("Sending message: {message:#?}");
        self.messages.lock().unwrap().push(message);
        Ok(())
    }

    fn read_from_camera(&self, camera_position: CameraPosition) -> Result<Image> {
        unimplemented!()
    }
}

impl Interfake {
    fn outgoing_messages(&self) -> Vec<OutgoingMessage> {
        take(&mut self.messages.lock().unwrap())
    }
}
