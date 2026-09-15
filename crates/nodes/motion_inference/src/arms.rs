use std::time::Duration;

use anyhow::{Result, ensure};
use booster::MotorCommand;
use kinematics::joints::{Joints, body::UpperBodyJoints};
use types::joint_limits::JointLimits;

use crate::{
    config::{Parameters, Policy, clip_measurement},
    inference::{joints_are_finite, position_targets},
    locomotion::leg,
};

/// Uses validated parameters and global joint limits. The caller owns the initial arm positions
/// and blend elapsed time, independently of inference policy changes.
pub fn generate_walking_arm_joints(
    position: &Joints<f32>,
    initial_position: UpperBodyJoints<f32>,
    elapsed: Duration,
    parameters: &Parameters,
    joints: &JointLimits,
) -> Result<UpperBodyJoints<MotorCommand>> {
    let position = clip_measurement(*position, joints.position);
    let ratio = (elapsed.as_secs_f32() / parameters.timing.arm_blend_duration.as_secs_f32())
        .clamp(0.0, 1.0);
    let locomotion = &parameters.locomotion;
    let mut target = Joints::fill(0.0);
    for (left, arm, initial, sign) in [
        (true, &mut target.left_arm, initial_position.left_arm, 1.0),
        (
            false,
            &mut target.right_arm,
            initial_position.right_arm,
            -1.0,
        ),
    ] {
        let (sole, knee) = leg(&position, left);
        arm.shoulder_pitch = sole.x() * locomotion.shoulder_pitch_scale;
        arm.shoulder_roll = sign
            * (locomotion.shoulder_roll_degrees.to_radians()
                + (sign * knee.y() - locomotion.knee_lateral_offset).max(0.0)
                    * locomotion.shoulder_roll_scale);
        arm.shoulder_yaw = 0.0;
        arm.elbow = sign
            * (locomotion.elbow_degrees.to_radians()
                + sole.x() * locomotion.shoulder_pitch_scale * locomotion.elbow_scale);
        *arm = initial * (1.0 - ratio) + *arm * ratio;
    }
    let (kp, kd) = Policy::Walk.gains(parameters);
    let joints = position_targets(target, kp, kd);
    ensure!(joints_are_finite(joints), "non-finite generated arm joints");
    Ok(UpperBodyJoints {
        left_arm: joints.left_arm,
        right_arm: joints.right_arm,
    })
}
