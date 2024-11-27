use std::time::SystemTime;

use hardware::{
    ActuatorInterface, CameraInterface, NetworkInterface, PathsInterface, RecordingInterface,
    SpeakerInterface, TimeInterface,
};

use color_eyre::eyre::Result;

use types::{
    audio::SpeakerRequest,
    camera_position::CameraPosition,
    hardware::Paths,
    joints::Joints,
    led::Leds,
    messages::{IncomingMessage, OutgoingMessage},
    ycbcr422_image::YCbCr422Image,
};

pub trait HardwareInterface:
    ActuatorInterface
    + CameraInterface
    + NetworkInterface
    + PathsInterface
    + RecordingInterface
    + SpeakerInterface
    + TimeInterface
{
}

pub struct ExtractorHardwareInterface;

/// `write_to_actuators` is a noop during replay
impl ActuatorInterface for ExtractorHardwareInterface {
    fn write_to_actuators(
        &self,
        _positions: Joints<f32>,
        _stiffnesses: Joints<f32>,
        _leds: Leds,
    ) -> Result<()> {
        Ok(())
    }
}

/// `read_from_camera` is only executed in setup nodes, which are not executed during replay
impl CameraInterface for ExtractorHardwareInterface {
    fn read_from_camera(&self, _camera_position: CameraPosition) -> Result<YCbCr422Image> {
        panic!("replayer cannot produce data from hardware")
    }
}

/// `read_from_network` is only executed in setup nodes, which are not executed during replay
/// `write_to_network` is a noop during replay
impl NetworkInterface for ExtractorHardwareInterface {
    fn read_from_network(&self) -> Result<IncomingMessage> {
        panic!("failed to read from network during replay")
    }

    fn write_to_network(&self, _message: OutgoingMessage) -> Result<()> {
        Ok(())
    }
}

/// recording is not supported for replaying
impl RecordingInterface for ExtractorHardwareInterface {
    fn should_record(&self) -> bool {
        false
    }

    fn set_whether_to_record(&self, _enable: bool) {}
}

/// imagine does not produce speaker outputs
impl SpeakerInterface for ExtractorHardwareInterface {
    fn write_to_speakers(&self, _request: SpeakerRequest) {}
}

impl PathsInterface for ExtractorHardwareInterface {
    fn get_paths(&self) -> Paths {
        Paths {
            motions: "etc/motions".into(),
            neural_networks: "etc/neural_networks".into(),
            sounds: "etc/sounds".into(),
        }
    }
}

impl TimeInterface for ExtractorHardwareInterface {
    fn get_now(&self) -> SystemTime {
        SystemTime::now()
    }
}

impl HardwareInterface for ExtractorHardwareInterface {}
