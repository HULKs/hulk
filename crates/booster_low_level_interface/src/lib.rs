use nalgebra::{Isometry3, Quaternion, Unit, Vector3};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use tokio::sync::broadcast::{Receiver, Sender};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LowState {
    pub imu_state: ImuState,                   // IMU feedback.
    pub motor_state_parallel: Vec<MotorState>, // Parallel structure joint feedback.
    pub motor_state_serial: Vec<MotorState>,   // Serial structure joint feedback.
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImuState {
    #[serde(rename = "rpy")]
    /// Euler angle information（0 -> roll ,1 -> pitch ,2 -> yaw）
    pub roll_pitch_yaw: [f32; 3],
    /// Angular velocity information（0 -> x ,1 -> y ,2 -> z）
    pub gyro: [f32; 3],
    /// Acceleration information.（0 -> x ,1 -> y ,2 -> z）
    pub acc: [f32; 3],
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MotorState {
    #[serde(rename = "q")]
    /// Joint angle position (q), unit: rad.
    pub position: f32,
    #[serde(rename = "dq")]
    /// Joint angular velocity (dq), unit: rad/s.
    pub velocity: f32,
    #[serde(rename = "ddq")]
    /// Joint angular acceleration (ddq), unit: rad/s².
    pub acceleration: f32,
    #[serde(rename = "tau_est")]
    /// Joint torque (tau_est), unit: nm
    pub torque: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CommandType {
    Parallel,
    Serial,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LowCommand {
    #[serde(rename = "cmd_type")]
    pub command_type: CommandType,
    #[serde(rename = "motor_cmd")]
    pub motor_command: Vec<MotorCommand>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MotorCommand {
    #[serde(rename = "q")]
    /// Joint angle position, unit: rad.
    pub position: f32,
    #[serde(rename = "dq")]
    /// Joint angular velocity, unit: rad/s.  
    pub velocity: f32,
    #[serde(rename = "tau")]
    /// Joint torque, unit: nm
    pub torque: f32,
    /// Proportional coefficient.
    pub kp: f32,
    /// Gain coefficient.
    pub kd: f32,
    /// Weight, range [0, 1], specify the proportion of user set motor cmd is mixed with the original cmd sent by the internal controller, which is usually used for gradually move to a user custom motor state from internal controlled motor state. Weight 0 means fully controlled by internal controller, weight 1 means fully controlled by user sent cmds. This parameter is not working if in custom mode, as in custom mode, internal controller will send no motor cmds.
    pub weight: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum FallDownStateType {
    IsReady,
    IsFalling,
    HasFallen,
    IsGettingUp,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FallDownState {
    pub fall_down_state: FallDownStateType,
    /// Whether recovery (getting up) action is available
    pub is_recovery_available: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ButtonEventType {
    PressDown,
    PressUp,
    SingleClick,
    DoubleClick,
    TripleClick,
    LongPressStart,
    LongPressHold,
    LongPressEnd,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ButtonEventMsg {
    pub button: i64,
    pub event: ButtonEventType,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RemoteControllerState {
    /** This feature can be used in user programs to implement custom gamepad/controller button functionality.
    |type | code | description|
    |-|-|-|
    |NONE|  0 |no event |
    |AXIS | 0x600 | axis motion |
    |HAT | 0x602 | hat position change |
    |BUTTON_DOWN | 0x603 | button pressed |
    |BUTTON_UP | 0x604 | button released |
    |REMOVE | 0x606 | device has been removed |
    */
    pub event: u64, // refer to remarks

    #[serde(rename = "lx")]
    /// left stick horizontal direction, push left to -1, push right to 1
    pub left_joystick_x: f32,
    #[serde(rename = "ly")]
    /// left stick vertical direction, push front to -1, push back to 1
    pub left_joystick_y: f32,
    #[serde(rename = "rx")]
    /// right stick horizontal direction, push left to -1, push right to 1
    pub right_joystick_x: f32,
    #[serde(rename = "ry")]
    /// right stick vertical direction, push front to -1, push back to 1
    pub right_joystick_y: f32,

    pub a: bool,
    pub b: bool,
    pub x: bool,
    pub y: bool,
    #[serde(rename = "lb")]
    pub left_button: bool,
    #[serde(rename = "rb")]
    pub right_button: bool,
    #[serde(rename = "lt")]
    pub left_trigger: bool,
    #[serde(rename = "rt")]
    pub right_trigger: bool,
    #[serde(rename = "ls")]
    pub left_joystick: bool,
    #[serde(rename = "rs")]
    pub right_joystick: bool,
    pub back: bool,
    pub start: bool,

    #[serde(rename = "hat_c")]
    pub dpad_centered: bool,
    #[serde(rename = "hat_u")]
    pub dpad_up: bool,
    #[serde(rename = "hat_d")]
    pub dpad_down: bool,
    #[serde(rename = "hat_l")]
    pub dpad_left: bool,
    #[serde(rename = "hat_r")]
    pub dpad_right: bool,
    #[serde(rename = "hat_lu")]
    pub dpad_left_up: bool,
    #[serde(rename = "hat_ld")]
    pub dpad_left_down: bool,
    #[serde(rename = "hat_ru")]
    pub dpad_right_up: bool,
    #[serde(rename = "hat_rd")]
    pub dpad_right_: bool,
    pub reserved: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransformStamped {
    pub header: Header,
    pub child_frame_id: String,
    #[serde(
        serialize_with = "serialize_isometry3_to_transform",
        deserialize_with = "deserialize_transform_to_isometry3"
    )]
    pub transform: Isometry3<f64>,
}

fn deserialize_transform_to_isometry3<'de, D>(deserializer: D) -> Result<Isometry3<f64>, D::Error>
where
    D: Deserializer<'de>,
{
    Transform::deserialize(deserializer)
        .map(|transform| {
            Isometry3::<f64>::from_parts(
                nalgebra::Translation::from(transform.translation),
                Unit::from_quaternion(transform.rotation),
            )
        })
        .map_err(serde::de::Error::custom)
}

fn serialize_isometry3_to_transform<S>(
    isometry: &Isometry3<f64>,
    serializer: S,
) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    let transform = Transform {
        translation: isometry.translation.vector,
        rotation: *isometry.rotation,
    };

    transform.serialize(serializer)
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Header {
    pub stamp: Time,
    pub frame_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Transform {
    pub translation: Vector3<f64>,
    pub rotation: Quaternion<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Time {
    #[serde(rename = "sec")]
    pub seconds: i32,
    #[serde(rename = "nanosec")]
    pub nanos: u32,
}

pub trait BoosterLowLevelInterface {
    fn subscribe_low_state(&self) -> Receiver<LowState>;

    fn publish_joint_ctrl(&self) -> Sender<LowCommand>;

    fn subscribe_fall_down(&self) -> Receiver<FallDownState>;

    fn subscribe_button_event(&self) -> Receiver<ButtonEventMsg>;

    fn subscribe_remote_controller_state(&self) -> Receiver<RemoteControllerState>;

    fn subscribe_frame_transform(&self) -> Receiver<TransformStamped>;
}
