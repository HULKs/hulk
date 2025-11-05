use hardware::{
    ActuatorInterface, LowCommandInterface, LowStateInterface, NetworkInterface, PathsInterface,
    RecordingInterface, SpeakerInterface, TimeInterface,
};

use color_eyre::eyre::Result;

use hula_types::hardware::Paths;
use types::{
    audio::SpeakerRequest,
    joints::Joints,
    led::Leds,
    messages::{IncomingMessage, OutgoingMessage},
};

pub trait HardwareInterface:
    ActuatorInterface
    + LowCommandInterface
    + LowStateInterface
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

impl LowCommandInterface for ExtractorHardwareInterface {
    fn write_low_command(&self, _low_command: booster::LowCommand) -> Result<()> {
        unimplemented!()
    }
}

impl LowStateInterface for ExtractorHardwareInterface {
    fn read_low_state(&self) -> Result<booster::LowState> {
        unimplemented!()
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
    fn get_now(&self) -> std::time::SystemTime {
        unimplemented!()
    }
}

impl HardwareInterface for ExtractorHardwareInterface {}
