use nalgebra::{vector, Isometry3, Translation, Vector3};

use crate::{
    inertial_measurement_unit::InertialMeasurementUnitData, robot_kinematics::RobotKinematics,
    support_foot::SupportFoot,
};

#[derive(Debug)]
pub struct Ground {
    pub robot_to_ground: Isometry3<f32>,
    pub ground_to_robot: Isometry3<f32>,
}

impl From<(InertialMeasurementUnitData, RobotKinematics, SupportFoot)> for Ground {
    fn from(
        (inertial_measurement_unit, robot_kinematics, support_foot): (
            InertialMeasurementUnitData,
            RobotKinematics,
            SupportFoot,
        ),
    ) -> Self {
        let imu_roll_pitch = inertial_measurement_unit.roll_pitch;
        let imu_roll = imu_roll_pitch.x;
        let imu_pitch = imu_roll_pitch.y;

        let left_sole_to_robot = robot_kinematics.left_sole_to_robot;
        let imu_adjusted_robot_to_left_sole = Isometry3::rotation(Vector3::y() * imu_pitch)
            * Isometry3::rotation(Vector3::x() * imu_roll)
            * Isometry3::from(left_sole_to_robot.translation.inverse());

        let right_sole_to_robot = robot_kinematics.right_sole_to_robot;
        let imu_adjusted_robot_to_right_sole = Isometry3::rotation(Vector3::y() * imu_pitch)
            * Isometry3::rotation(Vector3::x() * imu_roll)
            * Isometry3::from(right_sole_to_robot.translation.inverse());

        let left_sole_to_right_sole =
            right_sole_to_robot.translation.vector - left_sole_to_robot.translation.vector;
        let left_sole_to_ground =
            0.5 * vector![left_sole_to_right_sole.x, left_sole_to_right_sole.y, 0.0];

        let robot_to_ground = match support_foot {
            SupportFoot::Left => {
                Translation::from(-left_sole_to_ground) * imu_adjusted_robot_to_left_sole
            }
            SupportFoot::Right => {
                Translation::from(left_sole_to_ground) * imu_adjusted_robot_to_right_sole
            }
        };

        Self {
            robot_to_ground,
            ground_to_robot: robot_to_ground.inverse(),
        }
    }
}
