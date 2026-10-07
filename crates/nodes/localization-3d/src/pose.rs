use booster::ImuState;
use coordinate_systems::{Local, Robot};
use kinematics::robot_kinematics::RobotKinematics;
use linear_algebra::{Isometry3, Orientation3};

pub fn initial_robot_to_local_from_imu(
    imu: &ImuState,
    kinematics: &RobotKinematics,
) -> Isometry3<Robot, Local> {
    let rpy = imu.roll_pitch_yaw.inner;
    let orientation = Orientation3::from_euler_angles(rpy.x, rpy.y, 0.0);
    let left = orientation.inner * kinematics.left_leg.sole_to_robot.inner.translation.vector;
    let right = orientation.inner * kinematics.right_leg.sole_to_robot.inner.translation.vector;
    Isometry3::from_parts(
        linear_algebra::vector![<Local>, 0.0, 0.0, -left.z.min(right.z)],
        orientation,
    )
}
