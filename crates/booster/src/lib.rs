use coordinate_systems::Robot;
use linear_algebra::{vector, Vector3};
use path_serde::{PathDeserialize, PathIntrospect, PathSerialize};
use pyo3::{pyclass, pymethods, pymodule};
use ros2::geometry_msgs::transform_stamped::TransformStamped;
use serde::{Deserialize, Serialize};
use types::{
    joints::{arm::ArmJoints, head::HeadJoints, leg::LegJoints, Joints},
    parameters::MotorCommandParameters,
};

#[pyclass(frozen)]
#[derive(
    Clone, Debug, Default, Serialize, Deserialize, PathSerialize, PathDeserialize, PathIntrospect,
)]
pub struct LowState {
    /// IMU feedback
    pub imu_state: ImuState,
    /// Parallel structure joint feedback
    pub motor_state_parallel: Vec<MotorState>,
    /// Serial structure joint feedback
    pub motor_state_serial: Vec<MotorState>,
}

#[pymethods]
impl LowState {
    #[new]
    pub fn new(
        imu_state: ImuState,
        motor_state_parallel: Vec<MotorState>,
        motor_state_serial: Vec<MotorState>,
    ) -> Self {
        Self {
            imu_state,
            motor_state_parallel,
            motor_state_serial,
        }
    }
}

impl LowState {
    pub fn joint_positions(&self) -> Joints {
        let ms = &self.motor_state_serial;
        if ms.len() != 22 {
            panic!("expected 22 motor states, got {}", ms.len());
        }

        let head_yaw = ms[0].position;
        let head_pitch = ms[1].position;
        let left_shoulder_pitch = ms[2].position;
        let left_shoulder_roll = ms[3].position;
        let left_shoulder_yaw = ms[4].position;
        let left_elbow = ms[5].position;
        let right_shoulder_pitch = ms[6].position;
        let right_shoulder_roll = ms[7].position;
        let right_shoulder_yaw = ms[8].position;
        let right_elbow = ms[9].position;
        let left_hip_pitch = ms[10].position;
        let left_hip_roll = ms[11].position;
        let left_hip_yaw = ms[12].position;
        let left_knee = ms[13].position;
        let left_ankle_up = ms[14].position;
        let left_ankle_down = ms[15].position;
        let right_hip_pitch = ms[16].position;
        let right_hip_roll = ms[17].position;
        let right_hip_yaw = ms[18].position;
        let right_knee = ms[19].position;
        let right_ankle_up = ms[20].position;
        let right_ankle_down = ms[21].position;

        Joints {
            head: HeadJoints {
                yaw: head_yaw,
                pitch: head_pitch,
            },
            left_arm: ArmJoints {
                shoulder_pitch: left_shoulder_pitch,
                shoulder_roll: left_shoulder_roll,
                shoulder_yaw: left_shoulder_yaw,
                elbow: left_elbow,
            },
            right_arm: ArmJoints {
                shoulder_pitch: right_shoulder_pitch,
                shoulder_roll: right_shoulder_roll,
                shoulder_yaw: right_shoulder_yaw,
                elbow: right_elbow,
            },
            left_leg: LegJoints {
                hip_pitch: left_hip_pitch,
                hip_yaw: left_hip_yaw,
                hip_roll: left_hip_roll,
                knee: left_knee,
                ankle_up: left_ankle_up,
                ankle_down: left_ankle_down,
            },
            right_leg: LegJoints {
                hip_pitch: right_hip_pitch,
                hip_yaw: right_hip_yaw,
                hip_roll: right_hip_roll,
                knee: right_knee,
                ankle_up: right_ankle_up,
                ankle_down: right_ankle_down,
            },
        }
    }

    pub fn joint_velocities(&self) -> Joints {
        let ms = &self.motor_state_serial;
        if ms.len() != 22 {
            panic!("expected 22 motor states, got {}", ms.len());
        }

        let head_yaw = ms[0].velocity;
        let head_pitch = ms[1].velocity;
        let left_shoulder_pitch = ms[2].velocity;
        let left_shoulder_roll = ms[3].velocity;
        let left_shoulder_yaw = ms[4].velocity;
        let left_elbow = ms[5].velocity;
        let right_shoulder_pitch = ms[6].velocity;
        let right_shoulder_roll = ms[7].velocity;
        let right_shoulder_yaw = ms[8].velocity;
        let right_elbow = ms[9].velocity;
        let left_hip_pitch = ms[10].velocity;
        let left_hip_roll = ms[11].velocity;
        let left_hip_yaw = ms[12].velocity;
        let left_knee = ms[13].velocity;
        let left_ankle_up = ms[14].velocity;
        let left_ankle_down = ms[15].velocity;
        let right_hip_pitch = ms[16].velocity;
        let right_hip_roll = ms[17].velocity;
        let right_hip_yaw = ms[18].velocity;
        let right_knee = ms[19].velocity;
        let right_ankle_up = ms[20].velocity;
        let right_ankle_down = ms[21].velocity;

        Joints {
            head: HeadJoints {
                yaw: head_yaw,
                pitch: head_pitch,
            },
            left_arm: ArmJoints {
                shoulder_pitch: left_shoulder_pitch,
                shoulder_roll: left_shoulder_roll,
                shoulder_yaw: left_shoulder_yaw,
                elbow: left_elbow,
            },
            right_arm: ArmJoints {
                shoulder_pitch: right_shoulder_pitch,
                shoulder_roll: right_shoulder_roll,
                shoulder_yaw: right_shoulder_yaw,
                elbow: right_elbow,
            },
            left_leg: LegJoints {
                hip_pitch: left_hip_pitch,
                hip_yaw: left_hip_yaw,
                hip_roll: left_hip_roll,
                knee: left_knee,
                ankle_up: left_ankle_up,
                ankle_down: left_ankle_down,
            },
            right_leg: LegJoints {
                hip_pitch: right_hip_pitch,
                hip_yaw: right_hip_yaw,
                hip_roll: right_hip_roll,
                knee: right_knee,
                ankle_up: right_ankle_up,
                ankle_down: right_ankle_down,
            },
        }
    }
}

#[pyclass(frozen)]
#[derive(
    Clone, Debug, Default, Serialize, Deserialize, PathSerialize, PathDeserialize, PathIntrospect,
)]
pub struct ImuState {
    #[serde(rename = "rpy")]
    /// Euler angle information（x -> roll, y -> pitch, z -> yaw）
    pub roll_pitch_yaw: Vector3<Robot>,
    /// Angular velocity information
    #[serde(rename = "gyro")]
    pub angular_velocity: Vector3<Robot>,
    /// Acceleration information
    #[serde(rename = "acc")]
    pub linear_acceleration: Vector3<Robot>,
}

#[pymethods]
impl ImuState {
    #[new]
    pub fn new(
        roll_pitch_yaw: [f32; 3],
        angular_velocity: [f32; 3],
        linear_acceleration: [f32; 3],
    ) -> Self {
        Self {
            roll_pitch_yaw: vector![roll_pitch_yaw[0], roll_pitch_yaw[1], roll_pitch_yaw[2]],
            angular_velocity: vector![
                angular_velocity[0],
                angular_velocity[1],
                angular_velocity[2]
            ],
            linear_acceleration: vector![
                linear_acceleration[0],
                linear_acceleration[1],
                linear_acceleration[2]
            ],
        }
    }
}

#[pyclass(frozen, get_all)]
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

#[pymethods]
impl MotorState {
    #[new]
    pub fn new(position: f32, velocity: f32, acceleration: f32, torque: f32) -> Self {
        Self {
            position,
            velocity,
            acceleration,
            torque,
        }
    }
}

#[pyclass(frozen, eq)]
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum CommandType {
    Parallel,
    Serial,
}

#[pyclass(frozen, get_all)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LowCommand {
    #[serde(rename = "cmd_type")]
    pub command_type: CommandType,
    #[serde(rename = "motor_cmd")]
    pub motor_commands: Vec<MotorCommand>,
}

impl LowCommand {
    pub fn new(
        joint_velocities: &Joints,
        motor_command_parameters: &MotorCommandParameters,
    ) -> Self {
        LowCommand {
            command_type: CommandType::Serial,
            motor_commands: joint_velocities
                .into_iter()
                .zip(motor_command_parameters.proportional_coefficients)
                .zip(motor_command_parameters.derivative_coefficients)
                .map(|((joint_position, kp), kd)| MotorCommand {
                    position: joint_position,
                    velocity: 0.0,
                    torque: 0.0,
                    kp,
                    kd,
                    weight: 0.2,
                })
                .collect(),
        }
    }
}

#[pyclass(frozen, get_all)]
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

#[pymethods]
impl MotorCommand {
    #[new]
    pub fn new(position: f32, velocity: f32, torque: f32, kp: f32, kd: f32, weight: f32) -> Self {
        Self {
            position,
            velocity,
            torque,
            kp,
            kd,
            weight,
        }
    }
}

#[pyclass(frozen, get_all)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum FallDownStateType {
    IsReady,
    IsFalling,
    HasFallen,
    IsGettingUp,
}

#[pyclass(frozen, get_all)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FallDownState {
    pub fall_down_state: FallDownStateType,
    /// Whether recovery (getting up) action is available
    pub is_recovery_available: bool,
}

#[pymethods]
impl FallDownState {
    #[new]
    pub fn new(fall_down_state: FallDownStateType, is_recovery_available: bool) -> Self {
        Self {
            fall_down_state,
            is_recovery_available,
        }
    }
}

#[pyclass(frozen, get_all)]
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

#[pyclass(frozen, get_all)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ButtonEventMsg {
    pub button: i64,
    pub event: ButtonEventType,
}

#[pymethods]
impl ButtonEventMsg {
    #[new]
    pub fn new(button: i64, event: ButtonEventType) -> Self {
        Self { button, event }
    }
}

#[pyclass(frozen, get_all)]
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
    pub dpad_right_down: bool,
    pub reserved: u8,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename = "TFMessage")]
pub struct TransformMessage {
    pub transforms: Vec<TransformStamped>,
}

#[pymodule(name = "booster_types")]
pub mod python_module {

    #[pymodule_export]
    use crate::{
        ButtonEventMsg, ButtonEventType, CommandType, FallDownState, FallDownStateType, ImuState,
        LowCommand, LowState, MotorCommand, MotorState, RemoteControllerState,
    };
}
