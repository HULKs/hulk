use hardware::{NetworkInterface, RecordingInterface, SpeakerInterface, TimeInterface};
use interfake::FakeDataInterface;

pub mod fake_data;
pub mod interfake;
pub mod robot;
pub mod server;
pub mod simulator;
pub mod state;

include!(concat!(env!("OUT_DIR"), "/generated_code.rs"));

pub trait HardwareInterface:
    TimeInterface + NetworkInterface + RecordingInterface + FakeDataInterface + SpeakerInterface
{
}
