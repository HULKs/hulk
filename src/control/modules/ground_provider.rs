use macros::{module, require_some};
use nalgebra::{vector, Isometry3, Translation, Vector3};

use crate::types::{RobotKinematics, SensorData, Side, SupportFoot};

pub struct GroundProvider {}

#[module(control)]
#[input(path = sensor_data, data_type = SensorData)]
#[input(path = support_foot, data_type = SupportFoot)]
#[input(path = robot_kinematics, data_type = RobotKinematics)]
#[main_output(name = robot_to_ground, data_type = Isometry3<f32>)]
#[main_output(name = ground_to_robot, data_type = Isometry3<f32>)]
impl GroundProvider {}

impl GroundProvider {
    pub fn new() -> Self {
        Self {}
    }

    fn cycle(&mut self, context: CycleContext) -> anyhow::Result<MainOutputs> {
        let sensor_data = require_some!(context.sensor_data);
        let support_foot = require_some!(context.support_foot);
        let robot_kinematics = require_some!(context.robot_kinematics);
        let imu_roll_pitch = sensor_data.inertial_measurement_unit.roll_pitch;
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

        let robot_to_ground = match support_foot.support_side {
            Side::Left => Translation::from(-left_sole_to_ground) * imu_adjusted_robot_to_left_sole,
            Side::Right => {
                Translation::from(left_sole_to_ground) * imu_adjusted_robot_to_right_sole
            }
        };

        Ok(MainOutputs {
            robot_to_ground: Some(robot_to_ground),
            ground_to_robot: Some(robot_to_ground.inverse()),
        })
    }
}
