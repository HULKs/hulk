use std::{
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    time::SystemTime,
};

use ::hardware::{
    ActuatorInterface, CameraInterface, IdInterface, MicrophoneInterface, NetworkInterface,
    SensorInterface, TimeInterface,
};
use color_eyre::{
    eyre::{eyre, Error, WrapErr},
    Result,
};
use hardware::{PathsInterface, RecordingInterface, SpeakerInterface};
use parking_lot::Mutex;
use serde::Deserialize;
use spl_network::endpoint::{Endpoint, Ports};
use tokio::{
    runtime::{Builder, Runtime},
    select,
};
use tokio_util::sync::CancellationToken;
use types::{
    audio::SpeakerRequest,
    camera_position::CameraPosition,
    hardware::{Ids, Paths},
    joints::Joints,
    led::Leds,
    messages::{IncomingMessage, OutgoingMessage},
    samples::Samples,
    sensor_data::SensorData,
    ycbcr422_image::YCbCr422Image,
};

use super::{
    camera::Camera,
    hula_wrapper::HulaWrapper,
    microphones::{self, Microphones},
    speakers::{self, Speakers},
};

#[derive(Clone, Debug, Deserialize)]
pub struct Parameters {
    pub camera_top: nao_camera::Parameters,
    pub camera_bottom: nao_camera::Parameters,
    pub communication_addresses: Option<String>,
    pub microphones: microphones::Parameters,
    pub paths: Paths,
    pub speakers: speakers::Parameters,
    pub spl_network_ports: Ports,
}

pub struct HardwareInterface {
    hula_wrapper: Mutex<HulaWrapper>,
    microphones: Mutex<Microphones>,
    speakers: Speakers,
    paths: Paths,
    spl_network_endpoint: Endpoint,
    async_runtime: Runtime,
    camera_top: Mutex<Camera>,
    camera_bottom: Mutex<Camera>,
    enable_recording: AtomicBool,
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
            speakers: Speakers::new(parameters.speakers, &parameters.paths)
                .wrap_err("failed to initialize speakers")?,
            paths: parameters.paths,
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
            enable_recording: AtomicBool::new(false),
            keep_running,
        })
    }
}

impl ActuatorInterface for HardwareInterface {
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
}

impl CameraInterface for HardwareInterface {
    fn read_from_camera(&self, camera_position: CameraPosition) -> Result<YCbCr422Image> {
        match camera_position {
            CameraPosition::Top => self.camera_top.lock().read(),
            CameraPosition::Bottom => self.camera_bottom.lock().read(),
        }
    }
}

impl IdInterface for HardwareInterface {
    fn get_ids(&self) -> Ids {
        self.hula_wrapper.lock().get_ids()
    }
}

impl MicrophoneInterface for HardwareInterface {
    fn read_from_microphones(&self) -> Result<Samples> {
        self.microphones.lock().read_from_microphones()
    }
}

impl NetworkInterface for HardwareInterface {
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
}

impl PathsInterface for HardwareInterface {
    fn get_paths(&self) -> Paths {
        self.paths.clone()
    }
}

impl RecordingInterface for HardwareInterface {
    fn get_recording(&self) -> bool {
        self.enable_recording.load(Ordering::SeqCst)
    }

    fn set_recording(&self, enable: bool) {
        self.enable_recording.store(enable, Ordering::SeqCst)
    }
}

impl SensorInterface for HardwareInterface {
    fn read_from_sensors(&self) -> Result<SensorData> {
        self.hula_wrapper.lock().read_from_hula()
    }
}

impl SpeakerInterface for HardwareInterface {
    fn write_to_speakers(&self, request: SpeakerRequest) {
        self.speakers.write_to_speakers(request);
    }
}

impl TimeInterface for HardwareInterface {
    fn get_now(&self) -> SystemTime {
        self.hula_wrapper.lock().get_now()
    }
}

impl hulk::HardwareInterface for HardwareInterface {}
