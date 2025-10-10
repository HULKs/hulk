use coordinate_systems::Robot;
use linear_algebra::Vector3;
use path_serde::{PathDeserialize, PathIntrospect, PathSerialize};
use ros2::geometry_msgs::transform_stamped::TransformStamped;
use serde::{Deserialize, Serialize};
use types::{
    joints::{arm::ArmJoints, head::HeadJoints, leg::LegJoints, Joints},
    motor_commands::MotorCommands,
};

#[derive(
    Clone, Debug, Default, Serialize, Deserialize, PathSerialize, PathDeserialize, PathIntrospect,
)]
pub struct LowState {
    pub imu_state: ImuState,                   // IMU feedback.
    pub motor_state_parallel: Vec<MotorState>, // Parallel structure joint feedback.
    pub motor_state_serial: Vec<MotorState>,   // Serial structure joint feedback.
}

#[repr(C)]
struct BoosterJoints {
    head_yaw: MotorState,
    head_pitch: MotorState,
    left_shoulder_pitch: MotorState,
    left_shoulder_roll: MotorState,
    left_shoulder_yaw: MotorState,
    left_elbow: MotorState,
    right_shoulder_pitch: MotorState,
    right_shoulder_roll: MotorState,
    right_shoulder_yaw: MotorState,
    right_elbow: MotorState,
    left_hip_pitch: MotorState,
    left_hip_roll: MotorState,
    left_hip_yaw: MotorState,
    left_knee: MotorState,
    left_ankle_up: MotorState,
    left_ankle_down: MotorState,
    right_hip_pitch: MotorState,
    right_hip_roll: MotorState,
    right_hip_yaw: MotorState,
    right_knee: MotorState,
    right_ankle_up: MotorState,
    right_ankle_down: MotorState,
}

impl From<&LowState> for Joints<f32> {
    fn from(low_state: &LowState) -> Joints<f32> {
        let booster_serial_joints: BoosterJoints = unsafe {
            let try_into: [MotorState; 22] = low_state
                .motor_state_serial
                .clone()
                .try_into()
                .expect("failed to try into MotorState array");
            std::mem::transmute(try_into)
        };
        Joints::<f32> {
            head: HeadJoints::<f32> {
                yaw: booster_serial_joints.head_yaw.position,
                pitch: booster_serial_joints.head_pitch.position,
            },
            left_arm: ArmJoints::<f32> {
                shoulder_pitch: booster_serial_joints.left_shoulder_pitch.position,
                shoulder_roll: booster_serial_joints.left_shoulder_roll.position,
                elbow_yaw: booster_serial_joints.left_elbow.position,
                elbow_roll: 42.0,
                wrist_yaw: 42.0,
                hand: 42.0,
            },
            right_arm: ArmJoints::<f32> {
                shoulder_pitch: booster_serial_joints.right_shoulder_pitch.position,
                shoulder_roll: booster_serial_joints.right_shoulder_roll.position,
                elbow_yaw: booster_serial_joints.right_elbow.position,
                elbow_roll: 42.0,
                wrist_yaw: 42.0,
                hand: 42.0,
            },
            left_leg: LegJoints::<f32> {
                // Huh?! What is ankle_up down?
                ankle_pitch: booster_serial_joints.left_ankle_up.position,
                ankle_roll: booster_serial_joints.left_ankle_down.position,
                hip_pitch: booster_serial_joints.left_hip_pitch.position,
                hip_roll: booster_serial_joints.left_hip_roll.position,
                // Not quite analogous but good enough for now
                hip_yaw_pitch: booster_serial_joints.left_hip_yaw.position,

                knee_pitch: booster_serial_joints.left_knee.position,
            },
            right_leg: LegJoints::<f32> {
                // Huh?! What is ankle_up down?
                ankle_pitch: booster_serial_joints.right_ankle_up.position,
                ankle_roll: booster_serial_joints.right_ankle_down.position,
                hip_pitch: booster_serial_joints.right_hip_pitch.position,
                hip_roll: booster_serial_joints.right_hip_roll.position,
                // Not quite analogous but good enough for now
                hip_yaw_pitch: booster_serial_joints.right_hip_yaw.position,

                knee_pitch: booster_serial_joints.right_knee.position,
            },
        }
    }
}
#[derive(
    Clone, Debug, Default, Serialize, Deserialize, PathSerialize, PathDeserialize, PathIntrospect,
)]
pub struct ImuState {
    #[serde(rename = "rpy")]
    /// Euler angle information（0 -> roll ,1 -> pitch ,2 -> yaw）
    pub roll_pitch_yaw: Vector3<Robot>,
    /// Angular velocity information（0 -> x ,1 -> y ,2 -> z）
    #[serde(rename = "gyro")]
    pub angular_velocity: Vector3<Robot>,
    /// Acceleration information.（0 -> x ,1 -> y ,2 -> z）
    #[serde(rename = "acc")]
    pub linear_acceleration: Vector3<Robot>,
}

#[derive(Debug, Copy, Clone, Serialize, Deserialize)]
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
    pub motor_commands: Vec<MotorCommand>,
}

impl From<&MotorCommands<Joints>> for LowCommand {
    fn from(motor_commands: &MotorCommands<Joints>) -> Self {
        let joint_positions = vec![
            motor_commands.positions.head.yaw,
            motor_commands.positions.head.pitch,
            motor_commands.positions.left_arm.shoulder_pitch,
            motor_commands.positions.left_arm.shoulder_roll,
            // left_shoulder_yaw,
            0.0,
            motor_commands.positions.left_arm.elbow_yaw,
            motor_commands.positions.right_arm.shoulder_pitch,
            motor_commands.positions.right_arm.shoulder_roll,
            // right_shoulder_yaw,
            0.0,
            motor_commands.positions.right_arm.elbow_yaw,
            motor_commands.positions.left_leg.hip_pitch,
            motor_commands.positions.left_leg.hip_roll,
            motor_commands.positions.left_leg.hip_yaw_pitch,
            motor_commands.positions.left_leg.knee_pitch,
            motor_commands.positions.left_leg.ankle_pitch,
            motor_commands.positions.left_leg.ankle_roll,
            motor_commands.positions.right_leg.hip_pitch,
            motor_commands.positions.right_leg.hip_roll,
            motor_commands.positions.right_leg.hip_yaw_pitch,
            motor_commands.positions.right_leg.knee_pitch,
            motor_commands.positions.right_leg.ankle_pitch,
            motor_commands.positions.right_leg.ankle_roll,
        ];

        let joint_stiffnesses = vec![
            motor_commands.stiffnesses.head.yaw,
            motor_commands.stiffnesses.head.pitch,
            motor_commands.stiffnesses.left_arm.shoulder_pitch,
            motor_commands.stiffnesses.left_arm.shoulder_roll,
            // left_shoulder_yaw,
            0.0,
            motor_commands.stiffnesses.left_arm.elbow_yaw,
            motor_commands.stiffnesses.right_arm.shoulder_pitch,
            motor_commands.stiffnesses.right_arm.shoulder_roll,
            // right_shoulder_yaw,
            0.0,
            motor_commands.stiffnesses.right_arm.elbow_yaw,
            motor_commands.stiffnesses.left_leg.hip_pitch,
            motor_commands.stiffnesses.left_leg.hip_roll,
            motor_commands.stiffnesses.left_leg.hip_yaw_pitch,
            motor_commands.stiffnesses.left_leg.knee_pitch,
            motor_commands.stiffnesses.left_leg.ankle_pitch,
            motor_commands.stiffnesses.left_leg.ankle_roll,
            motor_commands.stiffnesses.right_leg.hip_pitch,
            motor_commands.stiffnesses.right_leg.hip_roll,
            motor_commands.stiffnesses.right_leg.hip_yaw_pitch,
            motor_commands.stiffnesses.right_leg.knee_pitch,
            motor_commands.stiffnesses.right_leg.ankle_pitch,
            motor_commands.stiffnesses.right_leg.ankle_roll,
        ];

        let motor_commands = joint_positions
            .into_iter()
            .zip(joint_stiffnesses)
            .map(|(position, stiffness)| MotorCommand {
                position,
                velocity: 0.0,
                torque: stiffness,
                kp: 40.0,
                kd: 0.1,
                weight: 1.0,
            })
            .collect();

        Self {
            command_type: CommandType::Serial,
            motor_commands,
        }
    }
}

#[derive(Debug, Default, Clone, Copy, Serialize, Deserialize)]
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

#[derive(Debug, Deserialize, Serialize)]
pub struct TFMessage {
    pub transforms: Vec<TransformStamped>,
}
