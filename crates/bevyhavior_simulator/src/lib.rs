#![allow(unexpected_cfgs)]

#[cfg(feature = "legacy-bevy-runner")]
use hardware::{
    CameraInterface, NetworkInterface, PathsInterface, RecordingInterface, SpeakerInterface,
    TimeInterface,
};
#[cfg(feature = "legacy-bevy-runner")]
use interfake::FakeDataInterface;

#[cfg(feature = "legacy-bevy-runner")]
pub mod autoref;
#[cfg(feature = "legacy-bevy-runner")]
pub mod ball;
pub mod behavior_tree_simulator;
#[cfg(feature = "legacy-bevy-runner")]
pub mod fake_data;
#[cfg(feature = "legacy-bevy-runner")]
pub mod field_dimensions;
#[cfg(feature = "legacy-bevy-runner")]
pub mod game_controller;
#[cfg(feature = "legacy-bevy-runner")]
pub mod interfake;
#[cfg(feature = "legacy-bevy-runner")]
pub mod recorder;
#[cfg(feature = "legacy-bevy-runner")]
pub mod robot;
#[cfg(feature = "legacy-bevy-runner")]
pub mod scenario;
#[cfg(feature = "legacy-bevy-runner")]
pub mod server;
#[cfg(feature = "legacy-bevy-runner")]
pub mod simulator;
#[cfg(feature = "legacy-bevy-runner")]
pub mod soft_error;
#[cfg(feature = "legacy-bevy-runner")]
pub mod test_rules;
#[cfg(feature = "legacy-bevy-runner")]
pub mod time;
#[cfg(feature = "legacy-bevy-runner")]
pub mod visual_referee;
#[cfg(feature = "legacy-bevy-runner")]
pub mod whistle;

#[cfg(feature = "legacy-bevy-runner")]
include!(concat!(env!("OUT_DIR"), "/generated_code.rs"));

#[cfg(feature = "legacy-bevy-runner")]
pub trait HardwareInterface:
    TimeInterface
    + NetworkInterface
    + RecordingInterface
    + FakeDataInterface
    + SpeakerInterface
    + PathsInterface
    + CameraInterface
{
}
