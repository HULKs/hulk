use nalgebra::Isometry3;
use serde::{Deserialize, Serialize};
use tokio::sync::broadcast::{Receiver, Sender};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LowState {
    pub imu_state: ImuState,                   // IMU feedback.
    pub motor_state_parallel: Vec<MotorState>, // Parallel structure joint feedback.
    pub motor_state_serial: Vec<MotorState>,   // Serial structure joint feedback.
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImuState {
    pub rpy: [f32; 3],  // Euler angle information（0 -> roll ,1 -> pitch ,2 -> yaw）
    pub gyro: [f32; 3], // Angular velocity information（0 -> x ,1 -> y ,2 -> z）
    pub acc: [f32; 3],  // Acceleration information.（0 -> x ,1 -> y ,2 -> z）
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MotorState {
    pub q: f32,       // Joint angle position, unit: rad.
    pub dq: f32,      // Joint angular velocity, unit: rad/s.
    pub ddq: f32,     // Joint angular acceleration, unit: rad/s².
    pub tau_est: f32, // Joint torque, unit: nm
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CmdType {
    Parallel, // Parallel type.
    Serial,   // Serial type.
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LowCmd {
    pub cmd_type: CmdType, // Set whether the joint command follows the serial mode or the parallel mode.
    pub motor_cmd: Vec<MotorCmd>, // Joint command array.
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MotorCmd {
    pub q: f32,      // Joint angle position, unit: rad.
    pub dq: f32,     // Joint angular velocity, unit: rad/s.
    pub tau: f32,    // Joint torque, unit: nm
    pub kp: f32,     // Proportional coefficient.
    pub kd: f32,     // Gain coefficient.
    pub weight: f32, // Weight, range [0, 1], specify the proportion of user set motor cmd is mixed with the original cmd sent by the internal controller, which is usually used for gradually move to a user custom motor state from internal controlled motor state. Weight 0 means fully controlled by internal controller, weight 1 means fully controlled by user sent cmds. This parameter is not working if in custom mode, as in custom mode, internal controller will send no motor cmds.
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum FallDownStateType {
    IsReady,     // Not fallen state
    IsFalling,   // Currently falling
    HasFallen,   // Already fallen
    IsGettingUp, // Currently getting up
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FallDownState {
    pub fall_down_state: FallDownStateType,
    pub is_recovery_available: bool, // Whether recovery (getting up) action is available
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
    pub event: u64, // refer to remarks

    pub lx: f32, // left stick horizontal direction, push left to -1, push right to 1
    pub ly: f32, // left stick vertical direction, push front to -1, push back to 1
    pub rx: f32, // right stick horizontal direction, push left to -1, push right to 1
    pub ry: f32, // right stick vertical direction, push front to -1, push back to 1

    pub a: bool,
    pub b: bool,
    pub x: bool,
    pub y: bool,
    pub lb: bool,
    pub rb: bool,
    pub lt: bool,
    pub rt: bool,
    pub ls: bool,
    pub rs: bool,
    pub back: bool,
    pub start: bool,

    pub hat_c: bool,  // Hat centered
    pub hat_u: bool,  // Hat up
    pub hat_d: bool,  // Hat down
    pub hat_l: bool,  // Hat left
    pub hat_r: bool,  // Hat right
    pub hat_lu: bool, // Hat left up
    pub hat_ld: bool, // Hat left down
    pub hat_ru: bool, // Hat right up
    pub hat_rd: bool, // Hat right down
    pub reserved: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransformStamped {
    pub header: Header,
    pub child_frame_id: String,
    pub transform: Isometry3<f32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Header {
    pub stamp: Time,
    pub frame_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Time {
    pub seconds: i32,
    pub nanos: u32,
}

pub trait BoosterLowLevelInterface {
    fn subscribe_low_state(&self) -> Receiver<LowState>;

    fn publish_joint_ctrl(&self) -> Sender<LowCmd>;

    fn subscribe_fall_down(&self) -> Receiver<FallDownState>;

    fn subscribe_button_event(&self) -> Receiver<ButtonEventMsg>;

    fn subscribe_remote_controller_state(&self) -> Receiver<RemoteControllerState>;

    fn subscribe_frame_transform(&self) -> Receiver<TransformStamped>;
}
