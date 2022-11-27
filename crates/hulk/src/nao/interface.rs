use std::{sync::Arc, time::SystemTime};

use color_eyre::{eyre::WrapErr, Result};
use parking_lot::Mutex;
use types::{
    hardware::{self, Ids, Image, Message, Samples},
    CameraPosition, Joints, Leds, SensorData,
};

use super::{
    camera::Camera, hula_wrapper::HulaWrapper, microphones::Microphones, parameters::Parameters,
};

pub struct Interface {
    hula_wrapper: Mutex<HulaWrapper>,
    microphones: Mutex<Microphones>,
    camera_top: Mutex<Camera>,
    camera_bottom: Mutex<Camera>,
}

impl Interface {
    pub fn new(parameters: Parameters) -> Result<Self> {
        let i2c_head_mutex = Arc::new(Mutex::new(()));

        Ok(Self {
            hula_wrapper: Mutex::new(
                HulaWrapper::new().wrap_err("failed to initialize HULA wrapper")?,
            ),
            microphones: Mutex::new(
                Microphones::new().wrap_err("failed to initialize microphones")?,
            ),
            camera_top: Mutex::new(
                Camera::new(
                    "/dev/video-top",
                    CameraPosition::Top,
                    parameters.camera_top,
                    i2c_head_mutex.clone(),
                )
                .wrap_err("failed to create top camera")?,
            ),
            camera_bottom: Mutex::new(
                Camera::new(
                    "/dev/video-bottom",
                    CameraPosition::Bottom,
                    parameters.camera_bottom,
                    i2c_head_mutex,
                )
                .wrap_err("failed to create bottom camera")?,
            ),
        })
    }
}

impl hardware::Interface for Interface {
    fn read_from_microphones(&self) -> Result<Samples> {
        self.microphones.lock().read_from_microphones()
    }

    fn get_now(&self) -> SystemTime {
        self.hula_wrapper.lock().get_now()
    }

    fn get_ids(&self) -> Ids {
        self.hula_wrapper.lock().get_ids()
    }

    fn read_from_sensors(&self) -> Result<SensorData> {
        self.hula_wrapper.lock().read_from_hula()
    }

    fn write_to_actuators(&self, positions: Joints, stiffnesses: Joints, leds: Leds) -> Result<()> {
        self.hula_wrapper
            .lock()
            .write_to_actuators(positions, stiffnesses, leds)
    }

    fn read_from_network(&self) -> Result<Message> {
        unimplemented!()
    }

    fn write_to_network(&self, _message: Message) -> Result<()> {
        unimplemented!()
    }

    fn read_from_camera(&self, camera_position: CameraPosition) -> Result<Image> {
        todo!()
    }
}
