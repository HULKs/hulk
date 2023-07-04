use std::time::Duration;

use framework::AdditionalOutput;
use nalgebra::Vector2;
use types::{
    parameters::WalkingEngine as WalkingEngineParameters, LegJoints, Side, StepAdjustment,
};

use super::{engine::non_continuous_quadratic_return, foot_offsets::FootOffsets};

pub fn support_leg_gyro_balancing(
    gyro: Vector2<f32>,
    gyro_balance_factors: LegJoints<f32>,
) -> LegJoints<f32> {
    LegJoints {
        ankle_pitch: gyro.y * gyro_balance_factors.ankle_pitch,
        ankle_roll: gyro.x * gyro_balance_factors.ankle_roll,
        hip_pitch: gyro.y * gyro_balance_factors.hip_pitch,
        hip_roll: gyro.x * gyro_balance_factors.hip_roll,
        hip_yaw_pitch: 0.0,
        knee_pitch: gyro.y * gyro_balance_factors.knee_pitch,
    }
}

#[allow(clippy::too_many_arguments)]
pub fn swing_leg_foot_leveling(
    left_leg: &LegJoints<f32>,
    right_leg: &LegJoints<f32>,
    measured_left_leg: LegJoints<f32>,
    measured_right_leg: LegJoints<f32>,
    torso_imu_pitch: f32,
    swing_side: Side,
    config: &WalkingEngineParameters,
    t: Duration,
    planned_step_duration: Duration,
) -> LegJoints<f32> {
    let support_leg = match swing_side {
        Side::Left => &right_leg,
        Side::Right => &left_leg,
    };
    let measured_support_leg = match swing_side {
        Side::Left => measured_right_leg,
        Side::Right => measured_left_leg,
    };

    let support_foot_pitch_error = measured_support_leg.ankle_pitch - support_leg.ankle_pitch;
    let pitch_error_adjustment =
        config.swing_foot_pitch_error_leveling_factor * support_foot_pitch_error;

    let imu_adjustment = config.swing_foot_imu_leveling_factor * torso_imu_pitch;
    let linear_time = (t.as_secs_f32() / planned_step_duration.as_secs_f32()).clamp(0.0, 1.0);

    LegJoints {
        ankle_pitch: (1.0 - linear_time) * (pitch_error_adjustment - imu_adjustment),
        ..Default::default()
    }
}

#[allow(clippy::too_many_arguments)]
pub fn step_adjustment(
    t: Duration,
    planned_step_duration: Duration,
    swing_side: Side,
    torso_tilt_shift: f32,
    current_left_foot: FootOffsets,
    current_right_foot: FootOffsets,
    next_left_walk_request: FootOffsets,
    next_right_walk_request: FootOffsets,
    last_left_walk_request: FootOffsets,
    last_right_walk_request: FootOffsets,
    forward_foot_support: f32,
    backward_foot_support: f32,
    max_adjustment: f32,
    step_adjustment_output: &mut AdditionalOutput<StepAdjustment>,
    left_foot_lift: f32,
    right_foot_lift: f32,
    stabilization_foot_lift_multiplier: f32,
    stabilization_foot_lift_offset: f32,
    remaining_stabilizing_steps: usize,
) -> (FootOffsets, FootOffsets, f32, f32, usize) {
    let linear_time = (t.as_secs_f32() / planned_step_duration.as_secs_f32()).clamp(0.0, 1.0);

    let left_delta = next_left_walk_request.forward - last_left_walk_request.forward;
    let right_delta = next_right_walk_request.forward - last_right_walk_request.forward;

    let next_left_forward = current_left_foot.forward + left_delta;
    let next_right_forward = current_right_foot.forward + right_delta;

    let backward_balance_limit =
        (next_left_forward).min(next_right_forward) + backward_foot_support;
    let forward_balance_limit = (next_left_forward).max(next_right_forward) + forward_foot_support;

    let (next_swing_foot, next_support_foot) = match swing_side {
        Side::Left => (next_left_forward, next_right_forward),
        Side::Right => (next_right_forward, next_left_forward),
    };
    let current_swing_foot = match swing_side {
        Side::Left => current_left_foot.forward,
        Side::Right => current_right_foot.forward,
    };

    let (adjusted_swing_foot, adjusted_support_foot) = if torso_tilt_shift < backward_balance_limit
    {
        let target = -torso_tilt_shift.abs() - backward_foot_support.abs();
        (
            target.clamp(
                next_swing_foot.min(current_swing_foot) - max_adjustment,
                next_swing_foot.max(current_swing_foot) + max_adjustment,
            ),
            0.0,
        )
    } else if torso_tilt_shift > forward_balance_limit {
        let target = torso_tilt_shift.abs() + forward_foot_support.abs();
        (
            target.clamp(
                next_swing_foot.min(current_swing_foot) - max_adjustment,
                next_swing_foot.max(current_swing_foot) + max_adjustment,
            ),
            0.0,
        )
    } else {
        (next_swing_foot, next_support_foot)
    };

    let ((adjusted_left_foot_lift, adjusted_right_foot_lift), adjusted_remaining_stabilizing_steps) =
        if adjusted_swing_foot - next_swing_foot != 0.0 {
            (
                (match swing_side {
                    Side::Left => (
                        left_foot_lift * stabilization_foot_lift_multiplier
                            + stabilization_foot_lift_offset
                                * non_continuous_quadratic_return(linear_time),
                        right_foot_lift,
                    ),
                    Side::Right => (
                        left_foot_lift,
                        right_foot_lift * stabilization_foot_lift_multiplier
                            + stabilization_foot_lift_offset
                                * non_continuous_quadratic_return(linear_time),
                    ),
                }),
                remaining_stabilizing_steps.max(1),
            )
        } else {
            (
                (left_foot_lift, right_foot_lift),
                remaining_stabilizing_steps,
            )
        };

    let (adjusted_left_forward, adjusted_right_forward) = match swing_side {
        Side::Left => (adjusted_swing_foot, adjusted_support_foot),
        Side::Right => (adjusted_support_foot, adjusted_swing_foot),
    };

    step_adjustment_output.fill_if_subscribed(|| StepAdjustment {
        adjusted_swing_foot,
        torso_tilt_shift,
        forward_balance_limit,
        backward_balance_limit,
        adjusted_left_foot_lift,
        adjusted_right_foot_lift,
    });
    (
        FootOffsets {
            forward: adjusted_left_forward,
            ..next_left_walk_request
        },
        FootOffsets {
            forward: adjusted_right_forward,
            ..next_right_walk_request
        },
        adjusted_left_foot_lift,
        adjusted_right_foot_lift,
        adjusted_remaining_stabilizing_steps,
    )
}
