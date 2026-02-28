use color_eyre::eyre::{bail, Result};
use coordinate_systems::Robot;
use linear_algebra::Vector3;
use path_serde::{PathDeserialize, PathIntrospect, PathSerialize};
use ros2::geometry_msgs::transform_stamped::TransformStamped;
use serde::{Deserialize, Serialize};
use types::{joints::Joints, parameters::MotorCommandParameters};

#[cfg(feature = "pyo3")]
use linear_algebra::vector;
#[cfg(feature = "pyo3")]
use pyo3::prelude::*;

#[repr(C)]
#[cfg_attr(feature = "pyo3", pyclass(frozen))]
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

#[cfg(feature = "pyo3")]
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
    pub fn serial_motor_states(&self) -> Result<Joints<MotorState>> {
        if self.motor_state_serial.len() == 22 {
            Ok(self
                .motor_state_serial
                .iter()
                .copied()
                .collect::<Joints<MotorState>>())
        } else {
            bail!("failed to construct motor states")
        }
    }

    pub fn parallel_motor_states(&self) -> Result<Joints<MotorState>> {
        if self.motor_state_parallel.len() == 22 {
            Ok(self
                .motor_state_parallel
                .iter()
                .copied()
                .collect::<Joints<MotorState>>())
        } else {
            bail!("failed to construct motor states")
        }
    }
}

#[repr(C)]
#[cfg_attr(feature = "pyo3", pyclass(frozen))]
#[derive(
    Clone,
    Copy,
    Debug,
    Default,
    PartialEq,
    Serialize,
    Deserialize,
    PathSerialize,
    PathDeserialize,
    PathIntrospect,
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

#[cfg(feature = "pyo3")]
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

#[repr(C)]
#[cfg_attr(feature = "pyo3", pyclass(frozen, get_all))]
#[derive(
    Debug,
    Default,
    Copy,
    Clone,
    PartialEq,
    Serialize,
    Deserialize,
    PathSerialize,
    PathDeserialize,
    PathIntrospect,
)]
pub struct MotorState {
    #[serde(rename = "mode")]
    /// Current motor command type used.
    pub command_type: CommandType,
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
    pub temperature: i8,
    pub lost: u32,
    pub reserve: [u32; 2],
}

#[allow(clippy::too_many_arguments)]
#[cfg(feature = "pyo3")]
#[pymethods]
impl MotorState {
    #[new]
    pub fn new(
        command_type: CommandType,
        position: f32,
        velocity: f32,
        acceleration: f32,
        torque: f32,
        temperature: i8,
        lost: u32,
        reserve: [u32; 2],
    ) -> Self {
        Self {
            command_type,
            position,
            velocity,
            acceleration,
            torque,
            temperature,
            lost,
            reserve,
        }
    }
}

pub trait JointsMotorState {
    fn positions(&self) -> Joints;
    fn velocities(&self) -> Joints;
    fn accelerations(&self) -> Joints;
    fn torques(&self) -> Joints;
}

impl JointsMotorState for Joints<MotorState> {
    fn positions(&self) -> Joints {
        self.into_iter()
            .map(|motor_state| motor_state.position)
            .collect::<Joints<f32>>()
    }

    fn velocities(&self) -> Joints {
        self.into_iter()
            .map(|motor_state| motor_state.velocity)
            .collect::<Joints<f32>>()
    }

    fn accelerations(&self) -> Joints {
        self.into_iter()
            .map(|motor_state| motor_state.acceleration)
            .collect::<Joints<f32>>()
    }

    fn torques(&self) -> Joints {
        self.into_iter()
            .map(|motor_state| motor_state.torque)
            .collect::<Joints<f32>>()
    }
}

#[repr(i8)]
#[cfg_attr(feature = "pyo3", pyclass(frozen, eq))]
#[derive(
    Debug,
    Default,
    Clone,
    Copy,
    Serialize,
    Deserialize,
    PartialEq,
    Eq,
    PathSerialize,
    PathDeserialize,
    PathIntrospect,
)]
pub enum CommandType {
    Parallel = 0,
    #[default]
    Serial = 1,
}

#[repr(C)]
#[cfg_attr(feature = "pyo3", pyclass(frozen, get_all))]
#[derive(
    Debug, Default, Clone, Serialize, Deserialize, PathSerialize, PathDeserialize, PathIntrospect,
)]
pub struct LowCommand {
    #[serde(rename = "cmd_type")]
    pub command_type: CommandType,
    #[serde(rename = "motor_cmd")]
    pub motor_commands: Vec<MotorCommand>,
}

impl LowCommand {
    pub fn new(
        joint_positions: &Joints,
        motor_command_parameters: &MotorCommandParameters,
        command_type: CommandType,
    ) -> Self {
        LowCommand {
            command_type,
            motor_commands: joint_positions
                .into_iter()
                .zip(motor_command_parameters.proportional_coefficients)
                .zip(motor_command_parameters.derivative_coefficients)
                .map(|((joint_position, kp), kd)| MotorCommand {
                    command_type,
                    position: joint_position,
                    velocity: 0.0,
                    torque: 0.0,
                    kp,
                    kd,
                    weight: motor_command_parameters.weight,
                })
                .collect(),
        }
    }
}

#[repr(C)]
#[cfg_attr(feature = "pyo3", pyclass(frozen, get_all))]
#[derive(
    Debug,
    Default,
    Clone,
    Copy,
    Serialize,
    Deserialize,
    PathSerialize,
    PathDeserialize,
    PathIntrospect,
)]
pub struct MotorCommand {
    #[serde(rename = "mode")]
    /// Current motor command type used.
    pub command_type: CommandType,
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

#[cfg(feature = "pyo3")]
#[pymethods]
impl MotorCommand {
    #[new]
    pub fn new(
        command_type: CommandType,
        position: f32,
        velocity: f32,
        torque: f32,
        kp: f32,
        kd: f32,
        weight: f32,
    ) -> Self {
        Self {
            command_type,
            position,
            velocity,
            torque,
            kp,
            kd,
            weight,
        }
    }
}

#[repr(u32)]
#[cfg_attr(feature = "pyo3", pyclass(frozen, get_all))]
#[derive(Debug, Clone, Serialize, Deserialize, PathSerialize, PathDeserialize, PathIntrospect)]
pub enum FallDownStateType {
    IsReady = 0,
    IsFalling = 1,
    HasFallen = 2,
    IsGettingUp = 3,
}

#[repr(C)]
#[cfg_attr(feature = "pyo3", pyclass(frozen, get_all))]
#[derive(Debug, Clone, Serialize, Deserialize, PathSerialize, PathDeserialize, PathIntrospect)]
pub struct FallDownState {
    pub fall_down_state: FallDownStateType,
    /// Whether recovery (getting up) action is available
    pub is_recovery_available: bool,
}

#[cfg(feature = "pyo3")]
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

#[repr(i8)]
#[cfg_attr(feature = "pyo3", pyclass(frozen, get_all))]
#[derive(Debug, Clone, Serialize, Deserialize, PathSerialize, PathDeserialize, PathIntrospect)]
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

#[repr(C)]
#[cfg_attr(feature = "pyo3", pyclass(frozen, get_all))]
#[derive(Debug, Clone, Serialize, Deserialize, PathSerialize, PathDeserialize, PathIntrospect)]
pub struct ButtonEventMsg {
    pub button: i32,
    pub event: ButtonEventType,
}

#[cfg(feature = "pyo3")]
#[pymethods]
impl ButtonEventMsg {
    #[new]
    pub fn new(button: i32, event: ButtonEventType) -> Self {
        Self { button, event }
    }
}

#[repr(C)]
#[cfg_attr(feature = "pyo3", pyclass(frozen, get_all))]
#[derive(
    Debug, Clone, Default, Serialize, Deserialize, PathSerialize, PathDeserialize, PathIntrospect,
)]
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
    pub event: u32, // refer to remarks

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

#[repr(C)]
#[derive(
    Clone, Debug, Default, Deserialize, Serialize, PathSerialize, PathDeserialize, PathIntrospect,
)]
#[serde(rename = "TFMessage")]
pub struct TransformMessage {
    pub transforms: Vec<TransformStamped>,
}

#[cfg(feature = "pyo3")]
#[pymodule(name = "booster_types")]
pub mod python_module {

    #[pymodule_export]
    use crate::{
        ButtonEventMsg, ButtonEventType, CommandType, FallDownState, FallDownStateType, ImuState,
        LowCommand, LowState, MotorCommand, MotorState, RemoteControllerState,
    };
}
