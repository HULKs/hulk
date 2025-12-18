use std::time::SystemTime;

use booster::{
    ButtonEventMsg, FallDownState, LowCommand, LowState, RemoteControllerState, TransformMessage,
};
use color_eyre::eyre::Result;

use hula_types::hardware::{Ids, Paths};
use types::{
    audio::SpeakerRequest,
    joints::Joints,
    led::Leds,
    messages::{IncomingMessage, OutgoingMessage},
    samples::Samples,
    sensor_data::SensorData,
};
use zed::RGBDSensors;

pub trait ActuatorInterface {
    fn write_to_actuators(
        &self,
        positions: Joints<f32>,
        stiffnesses: Joints<f32>,
        leds: Leds,
    ) -> Result<()>;
}

pub trait CameraInterface {
    fn read_rgbd_sensors(&self) -> Result<RGBDSensors>;
}

pub trait IdInterface {
    fn get_ids(&self) -> Ids;
}

pub trait MicrophoneInterface {
    fn read_from_microphones(&self) -> Result<Samples>;
}

pub trait NetworkInterface {
    fn read_from_network(&self) -> Result<IncomingMessage>;
    fn write_to_network(&self, message: OutgoingMessage) -> Result<()>;
}

pub trait PathsInterface {
    fn get_paths(&self) -> Paths;
}

pub trait RecordingInterface {
    fn should_record(&self) -> bool;
    fn set_whether_to_record(&self, enable: bool);
}

pub trait SensorInterface {
    fn read_from_sensors(&self) -> Result<SensorData>;
}

pub trait LowStateInterface {
    fn read_low_state(&self) -> Result<LowState>;
}

pub trait LowCommandInterface {
    fn write_low_command(&self, low_command: LowCommand) -> Result<()>;
}

pub trait FallDownStateInterface {
    fn read_fall_down_state(&self) -> Result<FallDownState>;
}
pub trait ButtonEventMsgInterface {
    fn read_button_event_msg(&self) -> Result<ButtonEventMsg>;
}
pub trait RemoteControllerStateInterface {
    fn read_remote_controller_state(&self) -> Result<RemoteControllerState>;
}
pub trait TransformMessageInterface {
    fn read_transform_message(&self) -> Result<TransformMessage>;
}

pub trait SpeakerInterface {
    fn write_to_speakers(&self, request: SpeakerRequest);
}

pub trait TimeInterface {
    fn get_now(&self) -> SystemTime;
}
