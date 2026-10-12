use kinematics::{
    forward::{left_sole_to_robot, right_sole_to_robot},
    joints::Joints,
};
use linear_algebra::Point3;

use crate::{FallenParameters, Observation, StandUpParameters, UprightParameters};

/// Standing on extended legs. A level torso alone also matches sitting or kneeling.
pub(super) fn is_standing(observation: &Observation, parameters: &UprightParameters) -> bool {
    let positions = observation.positions;
    let velocities = observation.velocities;
    // Robot-relative kinematics, needing neither ground nor odometry estimates.
    let left_height = -(left_sole_to_robot(&positions.left_leg) * Point3::origin()).z();
    let right_height = -(right_sole_to_robot(&positions.right_leg) * Point3::origin()).z();
    observation.tilt < parameters.maximum_torso_tilt
        && observation.angular_speed < parameters.maximum_torso_angular_speed
        && velocities
            .left_leg
            .into_iter()
            .chain(velocities.right_leg)
            .all(|velocity| velocity.abs() < parameters.maximum_leg_joint_speed)
        && left_height > parameters.minimum_torso_height_above_soles
        && right_height > parameters.minimum_torso_height_above_soles
}

pub(super) fn is_lying_still(observation: &Observation, parameters: &FallenParameters) -> bool {
    observation.tilt >= parameters.minimum_torso_tilt
        && observation.angular_speed < parameters.maximum_torso_angular_speed
        && without_head(observation.velocities)
            .all(|velocity| velocity.abs() < parameters.maximum_joint_speed)
}

/// Compare with the configured Prepare reference. This is a pose envelope,
/// not proof of collision clearance or membership in the policy's training set.
pub(super) fn is_in_start_pose(observation: &Observation, parameters: &StandUpParameters) -> bool {
    without_head(observation.positions - parameters.start_pose)
        .all(|deviation| deviation.abs() < parameters.maximum_start_pose_deviation)
        && without_head(observation.velocities)
            .all(|velocity| velocity.abs() < parameters.maximum_start_joint_speed)
}

/// The head does not affect falling or standing up.
fn without_head(joints: Joints<f32>) -> impl Iterator<Item = f32> {
    joints
        .left_arm
        .into_iter()
        .chain(joints.right_arm)
        .chain(joints.left_leg)
        .chain(joints.right_leg)
}
