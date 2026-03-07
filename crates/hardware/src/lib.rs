use std::time::SystemTime;

use booster::{
    ButtonEventMsg, FallDownState, Kick, LowCommand, LowState, RemoteControllerState,
    TransformMessage,
};
use booster_sdk::types::RobotMode;
use color_eyre::eyre::Result;

use hula_types::hardware::{Ids, Paths};
use kinematics::joints::{Joints, head::HeadJoints};
use ros2::sensor_msgs::{camera_info::CameraInfo, image::Image};
use types::{
    audio::SpeakerRequest,
    led::Leds,
    messages::{IncomingMessage, OutgoingMessage},
    motion_runtime::MotionRuntime,
    samples::Samples,
    sensor_data::SensorData,
    step::Step,
};

pub trait ActuatorInterface {
    fn write_to_actuators(
        &self,
        positions: Joints<f32>,
        stiffnesses: Joints<f32>,
        leds: Leds,
    ) -> Result<()>;
}

pub trait CameraInterface {
    fn read_rectified_image(&self) -> Result<Image>;
    fn read_stereonet_depth_image(&self) -> Result<Image>;
    fn read_stereonet_depth_camera_info(&self) -> Result<CameraInfo>;
    fn read_image_left_raw(&self) -> Result<Image>;
    fn read_image_left_raw_camera_info(&self) -> Result<CameraInfo>;
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

pub trait VisualKickInterface {
    fn write_visual_kick(&self, kick: Kick) -> Result<()>;
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

pub trait SimulatorInterface {
    fn is_simulation(&self) -> Result<bool>;
}

pub trait HighLevelInterface {
    fn change_mode(&self, mode: RobotMode) -> Result<()>;
    fn get_mode(&self) -> Result<RobotMode>;
    fn move_robot(&self, step: Step) -> Result<()>;
    fn rotate_head(&self, head_joints: HeadJoints<f32>) -> Result<()>;
    fn rotate_head_with_direction(&self, head_joints: HeadJoints<i32>) -> Result<()>;
    fn lie_down(&self) -> Result<()>;
    fn get_up(&self) -> Result<()>;
    fn get_up_with_mode(&self, mode: RobotMode) -> Result<()>;
    fn enter_wbc_gait(&self) -> Result<()>;
    fn exit_wbc_gait(&self) -> Result<()>;
    fn visual_kick(&self, start: bool) -> Result<()>;
}

pub trait MotionRuntimeInteface {
    fn get_motion_runtime_type(&self) -> Result<MotionRuntime>;
}
