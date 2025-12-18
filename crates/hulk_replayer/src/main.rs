#![recursion_limit = "256"]
mod controls;
mod coordinate_systems;
mod frames;
mod labels;
mod replayer;
mod ticks;
mod timeline;
mod user_data;
mod window;
mod worker_thread;

use color_eyre::{eyre::Result, install};
use hardware::{
    ActuatorInterface, CameraInterface, IdInterface, LowCommandInterface, LowStateInterface,
    MicrophoneInterface, NetworkInterface, PathsInterface, RecordingInterface, SensorInterface,
    SpeakerInterface, TimeInterface,
};
use hula_types::hardware::{Ids, Paths};
use replayer::replayer;
use types::{
    audio::SpeakerRequest,
    joints::Joints,
    led::Leds,
    messages::{IncomingMessage, OutgoingMessage},
    samples::Samples,
    sensor_data::SensorData,
};
use zed::RGBDSensors;

pub trait HardwareInterface:
    ActuatorInterface
    + CameraInterface
    + IdInterface
    + LowCommandInterface
    + LowStateInterface
    + MicrophoneInterface
    + NetworkInterface
    + PathsInterface
    + RecordingInterface
    + SensorInterface
    + SpeakerInterface
    + TimeInterface
{
}

include!(concat!(env!("OUT_DIR"), "/generated_code.rs"));

struct ReplayerHardwareInterface {
    ids: Ids,
}

impl ActuatorInterface for ReplayerHardwareInterface {
    fn write_to_actuators(
        &self,
        _positions: Joints<f32>,
        _stiffnesses: Joints<f32>,
        _leds: Leds,
    ) -> Result<()> {
        Ok(())
    }
}

impl CameraInterface for ReplayerHardwareInterface {
    fn read_rgbd_sensors(&self) -> Result<RGBDSensors> {
        unimplemented!("Replayer cannot produce data from hardware")
    }
}

impl IdInterface for ReplayerHardwareInterface {
    fn get_ids(&self) -> Ids {
        self.ids.clone()
    }
}

impl LowCommandInterface for ReplayerHardwareInterface {
    fn write_low_command(&self, _low_command: booster::LowCommand) -> Result<()> {
        unimplemented!()
    }
}

impl LowStateInterface for ReplayerHardwareInterface {
    fn read_low_state(&self) -> Result<booster::LowState> {
        unimplemented!()
    }
}

impl MicrophoneInterface for ReplayerHardwareInterface {
    fn read_from_microphones(&self) -> Result<Samples> {
        unimplemented!("Replayer cannot produce data from hardware")
    }
}

impl NetworkInterface for ReplayerHardwareInterface {
    fn read_from_network(&self) -> Result<IncomingMessage> {
        unimplemented!("Replayer cannot produce data from hardware")
    }

    fn write_to_network(&self, _message: OutgoingMessage) -> Result<()> {
        Ok(())
    }
}

impl PathsInterface for ReplayerHardwareInterface {
    fn get_paths(&self) -> Paths {
        Paths {
            motions: "etc/motions".into(),
            neural_networks: "etc/neural_networks".into(),
            sounds: "etc/sounds".into(),
        }
    }
}

impl RecordingInterface for ReplayerHardwareInterface {
    fn should_record(&self) -> bool {
        false
    }

    fn set_whether_to_record(&self, _enable: bool) {}
}

impl SensorInterface for ReplayerHardwareInterface {
    fn read_from_sensors(&self) -> Result<SensorData> {
        panic!("Replayer cannot produce data from hardware")
    }
}

impl SpeakerInterface for ReplayerHardwareInterface {
    fn write_to_speakers(&self, _request: SpeakerRequest) {}
}

impl TimeInterface for ReplayerHardwareInterface {
    fn get_now(&self) -> std::time::SystemTime {
        unimplemented!()
    }
}

impl HardwareInterface for ReplayerHardwareInterface {}

fn main() -> Result<()> {
    env_logger::init();
    install()?;
    replayer()
}
