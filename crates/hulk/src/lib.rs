#![recursion_limit = "256"]

use hardware::{
    ActuatorInterface, CameraInterface, IdInterface, MicrophoneInterface, NetworkInterface,
    SensorInterface, TimeInterface,
};

pub trait HardwareInterface:
    TimeInterface
    + SensorInterface
    + MicrophoneInterface
    + IdInterface
    + ActuatorInterface
    + NetworkInterface
    + CameraInterface
{
}

include!(concat!(env!("OUT_DIR"), "/generated_code.rs"));
