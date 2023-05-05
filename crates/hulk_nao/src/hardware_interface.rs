use std::{sync::Arc, time::SystemTime};

use color_eyre::{
    eyre::{eyre, Error, WrapErr},
    Result,
};
use parking_lot::Mutex;
use serde::Deserialize;
use tokio::{
    runtime::{Builder, Runtime},
    select,
};
use tokio_util::sync::CancellationToken;
use types::{
    hardware::{self, Ids},
    messages::{IncomingMessage, OutgoingMessage},
    samples::Samples,
    ycbcr422_image::YCbCr422Image,
    CameraPosition, Joints, Leds, SensorData,
};

use spl_network::endpoint::{Endpoint, Ports};

use super::{
    camera::Camera,
    hula_wrapper::HulaWrapper,
    microphones::{self, Microphones},
};

#[derive(Clone, Debug, Deserialize)]
pub struct Parameters {
    pub microphones: microphones::Parameters,
    pub spl_network_ports: Ports,
    pub camera_top: nao_camera::Parameters,
    pub camera_bottom: nao_camera::Parameters,
}

pub struct HardwareInterface {
    hula_wrapper: Mutex<HulaWrapper>,
    microphones: Mutex<Microphones>,
    spl_network_endpoint: Endpoint,
    async_runtime: Runtime,
    camera_top: Mutex<Camera>,
    camera_bottom: Mutex<Camera>,
    keep_running: CancellationToken,
}

impl HardwareInterface {
    pub fn new(keep_running: CancellationToken, parameters: Parameters) -> Result<Self> {
        let i2c_head_mutex = Arc::new(Mutex::new(()));
        let runtime = Builder::new_current_thread()
            .enable_all()
            .build()
            .wrap_err("failed to create tokio runtime")?;

        Ok(Self {
            hula_wrapper: Mutex::new(
                HulaWrapper::new().wrap_err("failed to initialize HULA wrapper")?,
            ),
            microphones: Mutex::new(
                Microphones::new(parameters.microphones)
                    .wrap_err("failed to initialize microphones")?,
            ),
            spl_network_endpoint: runtime
                .block_on(Endpoint::new(parameters.spl_network_ports))
                .wrap_err("failed to initialize SPL network")?,
            async_runtime: runtime,
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
            keep_running,
        })
    }
}

impl hardware::Interface for HardwareInterface {
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

    fn write_to_actuators(
        &self,
        positions: Joints<f32>,
        stiffnesses: Joints<f32>,
        leds: Leds,
    ) -> Result<()> {
        self.hula_wrapper
            .lock()
            .write_to_actuators(positions, stiffnesses, leds)
    }

    fn read_from_network(&self) -> Result<IncomingMessage> {
        self.async_runtime.block_on(async {
            select! {
                result =  self.spl_network_endpoint.read() => {
                    result.map_err(Error::from)
                },
                _ = self.keep_running.cancelled() => {
                    Err(eyre!("termination requested"))
                }
            }
        })
    }

    fn write_to_network(&self, message: OutgoingMessage) -> Result<()> {
        self.async_runtime
            .block_on(self.spl_network_endpoint.write(message));
        Ok(())
    }

    fn read_from_camera(&self, camera_position: CameraPosition) -> Result<YCbCr422Image> {
        match camera_position {
            CameraPosition::Top => self.camera_top.lock().read(),
            CameraPosition::Bottom => self.camera_bottom.lock().read(),
        }
    }
}
