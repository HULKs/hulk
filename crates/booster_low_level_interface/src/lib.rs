use nalgebra::Isometry3;
use tokio::sync::broadcast::{Receiver, Sender};

pub struct LowState {
    imu_state: ImuState,                   // IMU feedback.
    motor_state_parallel: Vec<MotorState>, // Parallel structure joint feedback.
    motor_state_serial: Vec<MotorState>,   // Serial structure joint feedback.
}

pub struct ImuState {
    rpy: [f32; 3],  // Euler angle information（0 -> roll ,1 -> pitch ,2 -> yaw）
    gyro: [f32; 3], // Angular velocity information（0 -> x ,1 -> y ,2 -> z）
    acc: [f32; 3],  // Acceleration information.（0 -> x ,1 -> y ,2 -> z）
}

pub struct MotorState {
    q: f32,       // Joint angle position, unit: rad.
    dq: f32,      // Joint angular velocity, unit: rad/s.
    ddq: f32,     // Joint angular acceleration, unit: rad/s².
    tau_est: f32, // Joint torque, unit: nm
}

pub enum CmdType {
    Parallel, // Parallel type.
    Serial,   // Serial type.
}

pub struct LowCmd {
    cmd_type: CmdType, // Set whether the joint command follows the serial mode or the parallel mode.
    motor_cmd: Vec<MotorCmd>, // Joint command array.
}

pub struct MotorCmd {
    q: f32,      // Joint angle position, unit: rad.
    dq: f32,     // Joint angular velocity, unit: rad/s.
    tau: f32,    // Joint torque, unit: nm
    kp: f32,     // Proportional coefficient.
    kd: f32,     // Gain coefficient.
    weight: f32, // Weight, range [0, 1], specify the proportion of user set motor cmd is mixed with the original cmd sent by the internal controller, which is usually used for gradually move to a user custom motor state from internal controlled motor state. Weight 0 means fully controlled by internal controller, weight 1 means fully controlled by user sent cmds. This parameter is not working if in custom mode, as in custom mode, internal controller will send no motor cmds.
}

pub enum FallDownStateType {
    IsReady,     // Not fallen state
    IsFalling,   // Currently falling
    HasFallen,   // Already fallen
    IsGettingUp, // Currently getting up
}

pub struct FallDownState {
    fall_down_state: FallDownStateType,
    is_recovery_available: bool, // Whether recovery (getting up) action is available
}

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

pub struct ButtonEventMsg {
    button: i64,
    event: ButtonEventType,
}

pub struct RemoteControllerState {
    event: u64, // refer to remarks

    lx: f32, // left stick horizontal direction, push left to -1, push right to 1
    ly: f32, // left stick vertical direction, push front to -1, push back to 1
    rx: f32, // right stick horizontal direction, push left to -1, push right to 1
    ry: f32, // right stick vertical direction, push front to -1, push back to 1

    a: bool,
    b: bool,
    x: bool,
    y: bool,
    lb: bool,
    rb: bool,
    lt: bool,
    rt: bool,
    ls: bool,
    rs: bool,
    back: bool,
    start: bool,

    hat_c: bool,  // Hat centered
    hat_u: bool,  // Hat up
    hat_d: bool,  // Hat down
    hat_l: bool,  // Hat left
    hat_r: bool,  // Hat right
    hat_lu: bool, // Hat left up
    hat_ld: bool, // Hat left down
    hat_ru: bool, // Hat right up
    hat_rd: bool, // Hat right down
    reserved: u8,
}

pub struct TransformStamped {
    header: Header,
    child_frame_id: String,
    transform: Isometry3<f32>,
}

pub struct Header {
    stamp: Time,
    frame_id: String,
}

pub struct Time {
    seconds: i32,
    nanos: u32,
}

pub trait BoosterLowLevelInterface {
    fn subscribe_low_state(&self) -> Receiver<LowState>;

    fn publish_joint_ctrl(&self) -> Sender<LowCmd>;

    fn subscribe_fall_down(&self) -> Receiver<FallDownState>;

    fn subscribe_button_event(&self) -> Receiver<ButtonEventMsg>;

    fn subscribe_remote_controller_state(&self) -> Receiver<RemoteControllerState>;

    fn subscribe_frame_transform(&self) -> Receiver<TransformStamped>;
}
