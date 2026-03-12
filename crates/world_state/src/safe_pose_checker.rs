use approx::AbsDiffEq;
use color_eyre::Result;
use kinematics::joints::Joints;
use serde::{Deserialize, Serialize};

use booster::{ImuState, JointsMotorState, MotorState};
use context_attribute::context;
use coordinate_systems::Robot;
use framework::{AdditionalOutput, MainOutput, PerceptionInput};
use linear_algebra::Vector3;

#[derive(Deserialize, Serialize)]
pub struct SafePoseChecker {
    pub last_imu_state: ImuState,
    pub last_serial_motor_states: Joints<MotorState>,
}

#[context]
pub struct CreationContext {}

#[context]
pub struct CycleContext {
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
}

#[context]
#[derive(Default)]
pub struct MainOutputs {
    pub is_safe_pose: MainOutput<bool>,
}

impl SafePoseChecker {
    pub fn new(_context: CreationContext) -> Result<Self> {
        Ok(Self {
            last_imu_state: Default::default(),
            last_serial_motor_states: Default::default(),
        })
    }

    pub fn cycle(&mut self, mut context: CycleContext) -> Result<MainOutputs> {
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

        Ok(MainOutputs {
            is_safe_pose: is_safe_pose.into(),
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
