use kinematics::forward::{left_sole_to_robot, right_sole_to_robot};
use linear_algebra::Point3;

use crate::{Observation, Parameters};

pub(super) fn is_upright(observation: &Observation, p: &Parameters) -> bool {
    let q = observation.positions;
    let reference = p.stand_up_pose;
    let legs_in_pose = [
        q.left_leg - reference.left_leg,
        q.right_leg - reference.right_leg,
    ]
    .into_iter()
    .all(|leg| {
        leg.into_iter()
            .zip(p.maximum_upright_leg_deviation)
            .all(|(deviation, maximum)| deviation.abs() < maximum)
    });
    observation.tilt < p.upright_tilt
        && observation.angular_speed < p.maximum_upright_angular_speed
        && legs_in_pose
        && observation.velocities.left_leg.into_iter().chain(observation.velocities.right_leg)
            .all(|velocity| velocity.abs() < p.maximum_upright_joint_speed)
        // A level torso while seated is not a standing configuration. This uses
        // robot-relative kinematics and needs neither ground nor odometry estimates.
        && (left_sole_to_robot(&q.left_leg) * Point3::origin()).z() < -p.minimum_upright_foot_depth
        && (right_sole_to_robot(&q.right_leg) * Point3::origin()).z() < -p.minimum_upright_foot_depth
}

pub(super) fn is_lying_still(observation: &Observation, p: &Parameters) -> bool {
    observation.tilt >= p.fallen_tilt
        && observation.angular_speed < p.maximum_fallen_angular_speed
        && observation
            .velocities
            .into_iter()
            .skip(2)
            .all(|velocity| velocity.abs() < p.maximum_fallen_joint_speed)
}
