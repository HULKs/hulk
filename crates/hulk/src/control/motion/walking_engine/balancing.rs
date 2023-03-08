use framework::AdditionalOutput;
use types::{
    configuration::WalkingEngine as WalkingEngineConfiguration, LegJoints, Side, StepAdjustment,
};

use super::foot_offsets::FootOffsets;

pub fn gyro_balancing(support_leg: &mut LegJoints, gyro_y: f32, gyro_balance_factor: f32) {
    let gyro_adjustment = gyro_balance_factor * gyro_y;
    support_leg.ankle_pitch += gyro_adjustment;
}

#[allow(clippy::too_many_arguments)]
pub fn foot_leveling(
    left_leg: &mut LegJoints,
    right_leg: &mut LegJoints,
    measured_left_leg: LegJoints,
    measured_right_leg: LegJoints,
    torso_imu_pitch: f32,
    swing_side: Side,
    last_left_level_adjustment: &mut f32,
    last_right_level_adjustment: &mut f32,
    config: &WalkingEngineConfiguration,
) {
    let support_leg = match swing_side {
        Side::Left => &right_leg,
        Side::Right => &left_leg,
    };
    let measured_support_leg = match swing_side {
        Side::Left => measured_right_leg,
        Side::Right => measured_left_leg,
    };

    let support_foot_pitch_error = measured_support_leg.ankle_pitch - support_leg.ankle_pitch;
    let mut swing_level_adjustment =
        config.swing_foot_pitch_error_leveling_factor * support_foot_pitch_error;

    // modify swing foot when tilted backwards
    if torso_imu_pitch.is_sign_negative() {
        swing_level_adjustment -= config.swing_foot_backwards_imu_leveling_factor * torso_imu_pitch;
    }

    let (left_level_adjustment, right_level_adjustment) = match swing_side {
        Side::Left => (swing_level_adjustment, 0.0),
        Side::Right => (0.0, swing_level_adjustment),
    };
    let limited_left_level_adjustment = *last_left_level_adjustment
        + (left_level_adjustment - *last_left_level_adjustment).clamp(
            -config.max_level_adjustment_velocity,
            config.max_level_adjustment_velocity,
        );
    *last_left_level_adjustment = limited_left_level_adjustment;
    let limited_right_level_adjustment = *last_right_level_adjustment
        + (right_level_adjustment - *last_right_level_adjustment).clamp(
            -config.max_level_adjustment_velocity,
            config.max_level_adjustment_velocity,
        );
    *last_right_level_adjustment = limited_right_level_adjustment;
    left_leg.ankle_pitch += limited_left_level_adjustment;
    right_leg.ankle_pitch += limited_right_level_adjustment;
}

#[allow(clippy::too_many_arguments)]
pub fn step_adjustment(
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
) -> (FootOffsets, FootOffsets) {
    let next_left_forward =
        current_left_foot.forward + next_left_walk_request.forward - last_left_walk_request.forward;
    let next_right_forward = current_right_foot.forward + next_right_walk_request.forward
        - last_right_walk_request.forward;
    let backward_balance_limit =
        (next_left_forward).min(next_right_forward) + backward_foot_support;
    let forward_balance_limit = (next_left_forward).max(next_right_forward) + forward_foot_support;
    let (next_swing_forward, next_support_forward) = match swing_side {
        Side::Left => (next_left_forward, next_right_forward),
        Side::Right => (next_right_forward, next_left_forward),
    };
    let adjustment = if torso_tilt_shift < backward_balance_limit {
        next_swing_forward - torso_tilt_shift - backward_balance_limit
    } else if torso_tilt_shift > forward_balance_limit {
        next_swing_forward - torso_tilt_shift - forward_balance_limit
    } else {
        0.0
    };
    let limited_adjustment = adjustment.clamp(-max_adjustment, max_adjustment);
    let adjusted_swing_forward = next_swing_forward - limited_adjustment;
    let adjusted_support_forward = next_support_forward + 0.5 * limited_adjustment;
    let (adjusted_left_forward, adjusted_right_forward) = match swing_side {
        Side::Left => (adjusted_swing_forward, adjusted_support_forward),
        Side::Right => (adjusted_support_forward, adjusted_swing_forward),
    };
    step_adjustment_output.fill_if_subscribed(|| StepAdjustment {
        adjustment,
        limited_adjustment,
        torso_tilt_shift,
        forward_balance_limit,
        backward_balance_limit,
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
    )
}
