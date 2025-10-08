use std::time::SystemTime;

use booster::{ButtonEventMsg, FallDownState, LowCommand, LowState, RemoteControllerState};
use color_eyre::eyre::Result;

use hula_types::hardware::{Ids, Paths};
use ros2::geometry_msgs::transform_stamped::TransformStamped;
use simulation_message::SimulationMessage;
use types::{
    audio::SpeakerRequest,
    camera_position::CameraPosition,
    joints::Joints,
    led::Leds,
    messages::{IncomingMessage, OutgoingMessage},
    samples::Samples,
    sensor_data::SensorData,
    ycbcr422_image::YCbCr422Image,
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
    fn read_from_camera(&self, camera_position: CameraPosition) -> Result<YCbCr422Image>;
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
    fn read_low_state(&self) -> Result<SimulationMessage<LowState>>;
}

pub trait LowCommandInterface {
    fn write_low_command(&self, low_command: LowCommand) -> Result<()>;
}

pub trait FallDownStateInterface {
    fn read_fall_down_state(&self) -> Result<SimulationMessage<FallDownState>>;
}
pub trait ButtonEventMsgInterface {
    fn read_button_event_msg(&self) -> Result<SimulationMessage<ButtonEventMsg>>;
}
pub trait RemoteControllerStateInterface {
    fn read_remote_controller_state(&self) -> Result<SimulationMessage<RemoteControllerState>>;
}
pub trait TransformStampedInterface {
    fn read_transform_stamped(&self) -> Result<SimulationMessage<TransformStamped>>;
}

pub trait RGBDSensorsInterface {
    fn read_rgbd_sensors(&self) -> Result<SimulationMessage<RGBDSensors>>;
}

pub trait SpeakerInterface {
    fn write_to_speakers(&self, request: SpeakerRequest);
}

pub trait TimeInterface {
    fn get_now(&self) -> SystemTime;
}
