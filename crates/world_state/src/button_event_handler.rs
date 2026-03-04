use color_eyre::Result;
use coordinate_systems::Robot;
use hardware::InjectedButtonInterface;
use linear_algebra::Vector3;
use serde::{Deserialize, Serialize};

use approx::AbsDiffEq;
use booster::{ButtonEventMsg, ButtonEventType, ImuState, JointsMotorState, MotorState};
use context_attribute::context;
use framework::{AdditionalOutput, MainOutput, PerceptionInput};
use types::{buttons::Buttons, joints::Joints};

#[derive(Deserialize, Serialize)]
pub struct ButtonEventHandler {
    pub last_imu_state: ImuState,
    pub last_serial_motor_states: Joints<MotorState>,
    pub last_button_event: Option<ButtonEventMsg>,
}

#[context]
pub struct CreationContext {}

#[context]
pub struct CycleContext {
    maybe_button_event: PerceptionInput<Option<ButtonEventMsg>, "ButtonEvent", "button_event?">,
    imu_state: PerceptionInput<ImuState, "Motion", "imu_state">,
    serial_motor_states: PerceptionInput<Joints<MotorState>, "Motion", "serial_motor_states">,

    prep_mode_serial_motor_states:
        Parameter<Joints<MotorState>, "safe_mode_handler.prep_mode_serial_motor_states">,
    prep_mode_imu_state: Parameter<ImuState, "safe_mode_handler.prep_mode_imu_state">,
    joint_position_threshold: Parameter<f32, "safe_mode_handler.joint_position_threshold">,
    joint_velocity_threshold: Parameter<f32, "safe_mode_handler.joint_velocity_threshold">,
    angular_velocity_threshold: Parameter<f32, "safe_mode_handler.angular_velocity_threshold">,
    linear_acceleration_threshold:
        Parameter<f32, "safe_mode_handler.linear_acceleration_threshold">,

    joint_position_difference_to_safe:
        AdditionalOutput<Joints, "joint_position_difference_to_safe">,
    joint_velocities_difference_to_safe:
        AdditionalOutput<Joints, "joint_velocities_difference_to_safe">,
    angular_velocities_difference_to_safe:
        AdditionalOutput<Vector3<Robot>, "angular_velocities_difference_to_safe">,
    linear_accelerations_difference_to_safe:
        AdditionalOutput<Vector3<Robot>, "linear_accelerations_difference_to_safe">,

    hardware_interface: HardwareInterface,
}

#[context]
#[derive(Default)]
pub struct MainOutputs {
    pub buttons: MainOutput<Option<Buttons>>,
}

impl ButtonEventHandler {
    pub fn new(_context: CreationContext) -> Result<Self> {
        Ok(Self {
            last_imu_state: Default::default(),
            last_serial_motor_states: Default::default(),
            last_button_event: None,
        })
    }

    pub fn cycle(
        &mut self,
        mut context: CycleContext<impl InjectedButtonInterface>,
    ) -> Result<MainOutputs> {
        let injected_buttons = context.hardware_interface.read_injected_button()?;

        if injected_buttons.is_some() {
            return Ok(MainOutputs {
                buttons: injected_buttons.into(),
            });
        }

        let Some(button_event) = context
            .maybe_button_event
            .persistent
            .into_iter()
            .flat_map(|(_time, info)| info)
            .last()
            .flatten()
        else {
            return Ok(MainOutputs {
                buttons: None.into(),
            });
        };

        let imu_state = context
            .imu_state
            .persistent
            .into_iter()
            .chain(context.imu_state.temporary)
            .flat_map(|(_time, info)| info)
            .last()
            .cloned()
            .unwrap_or(self.last_imu_state);

        self.last_imu_state = imu_state;

        let serial_motor_states = context
            .serial_motor_states
            .persistent
            .into_iter()
            .chain(context.serial_motor_states.temporary)
            .flat_map(|(_time, info)| info)
            .last()
            .cloned()
            .unwrap_or(self.last_serial_motor_states);

        self.last_serial_motor_states = serial_motor_states;

        context
            .joint_position_difference_to_safe
            .fill_if_subscribed(|| {
                serial_motor_states.positions() - context.prep_mode_serial_motor_states.positions()
            });
        context
            .joint_velocities_difference_to_safe
            .fill_if_subscribed(|| {
                serial_motor_states.velocities()
                    - context.prep_mode_serial_motor_states.velocities()
            });
        context
            .linear_accelerations_difference_to_safe
            .fill_if_subscribed(|| {
                imu_state.linear_acceleration - context.prep_mode_imu_state.linear_acceleration
            });
        context
            .angular_velocities_difference_to_safe
            .fill_if_subscribed(|| {
                imu_state.angular_velocity - context.prep_mode_imu_state.angular_velocity
            });

        let motor_states_are_safe = motor_states_are_safe(
            &serial_motor_states,
            context.prep_mode_serial_motor_states,
            *context.joint_position_threshold,
            *context.joint_velocity_threshold,
        );

        let imu_state_is_safe = imu_state_is_safe(
            &imu_state,
            context.prep_mode_imu_state,
            *context.angular_velocity_threshold,
            *context.linear_acceleration_threshold,
        );

        let is_safe_pose = motor_states_are_safe && imu_state_is_safe;

        let buttons = match (self.last_button_event.clone(), button_event, is_safe_pose) {
            (
                Some(ButtonEventMsg {
                    button: 1,
                    event: ButtonEventType::LongPressHold,
                }),
                ButtonEventMsg {
                    button: 1,
                    event: ButtonEventType::PressUp,
                },
                false,
            ) => Some(Buttons::IsStandLongPressed),
            (
                Some(ButtonEventMsg {
                    button: 1,
                    event: ButtonEventType::LongPressHold,
                }),
                ButtonEventMsg {
                    button: 1,
                    event: ButtonEventType::PressUp,
                },
                true,
            ) => Some(Buttons::IsStandLongPressedDuringSafePose),
            (
                _,
                ButtonEventMsg {
                    button: 0,
                    event: ButtonEventType::PressUp,
                }
                | ButtonEventMsg {
                    button: 1,
                    event: ButtonEventType::PressUp,
                },
                _,
            ) => Some(Buttons::IsStandOrF1Pressed),
            _ => None,
        };

        self.last_button_event = Some(button_event.clone());

        Ok(MainOutputs {
            buttons: buttons.into(),
        })
    }
}

fn motor_states_are_safe(
    serial_motor_states: &Joints<MotorState>,
    prep_mode_serial_motor_states: &Joints<MotorState>,
    joint_position_threshold: f32,
    joint_velocity_threshold: f32,
) -> bool {
    serial_motor_states
        .into_iter()
        .zip(*prep_mode_serial_motor_states)
        .all(|(current_motor_state, safe_motor_state)| {
            current_motor_state
                .position
                .abs_diff_eq(&safe_motor_state.position, joint_position_threshold)
                && current_motor_state
                    .velocity
                    .abs_diff_eq(&safe_motor_state.velocity, joint_velocity_threshold)
        })
}

fn imu_state_is_safe(
    imu_state: &ImuState,
    prep_mode_imu_state: &ImuState,
    angular_velocity_threshold: f32,
    linear_acceleration_threshold: f32,
) -> bool {
    imu_state.angular_velocity.abs_diff_eq(
        &prep_mode_imu_state.angular_velocity,
        angular_velocity_threshold,
    ) && imu_state.linear_acceleration.abs_diff_eq(
        &prep_mode_imu_state.linear_acceleration,
        linear_acceleration_threshold,
    )
}
