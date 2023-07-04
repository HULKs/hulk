#![recursion_limit = "256"]

use hardware::{
    ActuatorInterface, CameraInterface, IdInterface, MicrophoneInterface, NetworkInterface,
    PathsInterface, SensorInterface, TimeInterface,
};

pub trait HardwareInterface:
    ActuatorInterface
    + CameraInterface
    + IdInterface
    + MicrophoneInterface
    + PathsInterface
    + NetworkInterface
    + SensorInterface
    + TimeInterface
{
}

include!(concat!(env!("OUT_DIR"), "/generated_code.rs"));
