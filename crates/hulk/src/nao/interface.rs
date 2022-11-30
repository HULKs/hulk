use std::{sync::Arc, time::SystemTime};

use color_eyre::{eyre::WrapErr, Result};
use parking_lot::Mutex;
use tokio_util::sync::CancellationToken;
use types::{
    hardware::{self, Ids, Image, IncomingMessage, OutgoingMessage, Samples},
    CameraPosition, Joints, Leds, SensorData,
};

use crate::network::Network;

use super::{
    camera::Camera, hula_wrapper::HulaWrapper, microphones::Microphones, parameters::Parameters,
};

pub struct Interface {
    keep_running: CancellationToken,
    hula_wrapper: Mutex<HulaWrapper>,
    microphones: Mutex<Microphones>,
    network: Network,
    camera_top: Mutex<Camera>,
    camera_bottom: Mutex<Camera>,
}

impl Interface {
    pub fn new(keep_running: CancellationToken, parameters: Parameters) -> Result<Self> {
        let i2c_head_mutex = Arc::new(Mutex::new(()));

        Ok(Self {
            keep_running,
            hula_wrapper: Mutex::new(
                HulaWrapper::new().wrap_err("failed to initialize HULA wrapper")?,
            ),
            microphones: Mutex::new(
                Microphones::new(parameters.microphones)
                    .wrap_err("failed to initialize microphones")?,
            ),
            network: Network::new(keep_running.clone(), parameters.network)
                .wrap_err("failed to initialize network")?,
            camera_top: Mutex::new(
                Camera::new(
                    "/dev/video-top",
                    CameraPosition::Top,
                    parameters.camera_top,
                    i2c_head_mutex.clone(),
                )
                .wrap_err("failed to initialize top camera")?,
            ),
            camera_bottom: Mutex::new(
                Camera::new(
                    "/dev/video-bottom",
                    CameraPosition::Bottom,
                    parameters.camera_bottom,
                    i2c_head_mutex,
                )
                .wrap_err("failed to initialize bottom camera")?,
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

    fn read_from_network(&self) -> Result<IncomingMessage> {
        self.network.read()
    }

    fn write_to_network(&self, message: OutgoingMessage) -> Result<()> {
        self.network.write(message)
    }

    fn read_from_camera(&self, camera_position: CameraPosition) -> Result<Image> {
        match camera_position {
            CameraPosition::Top => self.camera_top.lock().read(),
            CameraPosition::Bottom => self.camera_bottom.lock().read(),
        }
    }
}
