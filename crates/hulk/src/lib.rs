#![recursion_limit = "256"]

use hardware::{
    ActuatorInterface, CameraInterface, IdInterface, MicrophoneInterface, NetworkInterface,
    PathsInterface, RecordingInterface, SensorInterface, SpeakerInterface, TimeInterface,
};

pub trait HardwareInterface:
    ActuatorInterface
    + CameraInterface
    + IdInterface
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
