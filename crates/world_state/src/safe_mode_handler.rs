use color_eyre::Result;
use serde::{Deserialize, Serialize};

use approx::AbsDiffEq;
use booster::{ButtonEventMsg, ButtonEventType, ImuState, MotorState};
use context_attribute::context;
use framework::{MainOutput, PerceptionInput};
use types::joints::Joints;

#[derive(Deserialize, Serialize)]
pub struct SafeModeHandler {}

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
}

#[context]
#[derive(Default)]
pub struct MainOutputs {
    pub safe_to_leave_safe_mode: MainOutput<bool>,
}

impl SafeModeHandler {
    pub fn new(_context: CreationContext) -> Result<Self> {
        Ok(Self {})
    }

    pub fn cycle(&mut self, context: CycleContext) -> Result<MainOutputs> {
        let Some(button_event) = context
            .maybe_button_event
            .persistent
            .into_iter()
            .chain(context.maybe_button_event.temporary)
            .flat_map(|(_time, info)| info)
            .last()
            .flatten()
        else {
            return Ok(MainOutputs {
                safe_to_leave_safe_mode: false.into(),
            });
        };

        let Some(imu_state) = context
            .imu_state
            .persistent
            .into_iter()
            .chain(context.imu_state.temporary)
            .flat_map(|(_time, info)| info)
            .last()
        else {
            return Ok(MainOutputs {
                safe_to_leave_safe_mode: false.into(),
            });
        };

        let Some(serial_motor_states) = context
            .serial_motor_states
            .persistent
            .into_iter()
            .chain(context.serial_motor_states.temporary)
            .flat_map(|(_time, info)| info)
            .last()
        else {
            return Ok(MainOutputs {
                safe_to_leave_safe_mode: false.into(),
            });
        };

        let is_stand_button_double_click = matches!(
            button_event,
            ButtonEventMsg {
                button: 1,
                event: ButtonEventType::DoubleClick
            }
        );

        let motor_states_are_safe = motor_states_are_safe(
            serial_motor_states,
            context.prep_mode_serial_motor_states,
            context.joint_position_threshold,
            context.joint_velocity_threshold,
        );

        let imu_state_is_safe = imu_state_is_safe(
            imu_state,
            context.prep_mode_imu_state,
            context.angular_velocity_threshold,
            context.linear_acceleration_threshold,
        );

        let safe_to_leave_safe_mode =
            is_stand_button_double_click && motor_states_are_safe && imu_state_is_safe;

        Ok(MainOutputs {
            safe_to_leave_safe_mode: safe_to_leave_safe_mode.into(),
        })
    }
}

fn motor_states_are_safe(
    serial_motor_states: &Joints<MotorState>,
    prep_mode_serial_motor_states: &Joints<MotorState>,
    joint_position_threshold: &f32,
    joint_velocity_threshold: &f32,
) -> bool {
    serial_motor_states
        .into_iter()
        .zip(*prep_mode_serial_motor_states)
        .all(|(current_motor_state, safe_motor_state)| {
            current_motor_state
                .position
                .abs_diff_eq(&safe_motor_state.position, *joint_position_threshold)
                && current_motor_state
                    .velocity
                    .abs_diff_eq(&safe_motor_state.velocity, *joint_velocity_threshold)
        })
}

fn imu_state_is_safe(
    imu_state: &ImuState,
    prep_mode_imu_state: &ImuState,
    angular_velocity_threshold: &f32,
    linear_acceleration_threshold: &f32,
) -> bool {
    imu_state.angular_velocity.abs_diff_eq(
        &prep_mode_imu_state.angular_velocity,
        *angular_velocity_threshold,
    ) && imu_state.linear_acceleration.abs_diff_eq(
        &prep_mode_imu_state.linear_acceleration,
        *linear_acceleration_threshold,
    )
}
