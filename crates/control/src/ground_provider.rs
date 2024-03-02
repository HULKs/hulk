use color_eyre::Result;
use serde::{Deserialize, Serialize};

use context_attribute::context;
use coordinate_systems::{vector, Isometry3, Vector3};
use framework::MainOutput;
use types::{
    coordinate_systems::{Ground, LeftSole, RightSole, Robot},
    robot_kinematics::RobotKinematics,
    sensor_data::SensorData,
    support_foot::{Side, SupportFoot},
};

#[derive(Deserialize, Serialize)]
pub struct GroundProvider {}

#[context]
pub struct CreationContext {}

#[context]
pub struct CycleContext {
    robot_kinematics: Input<RobotKinematics, "robot_kinematics">,
    sensor_data: Input<SensorData, "sensor_data">,
    support_foot: Input<SupportFoot, "support_foot">,
}

#[context]
#[derive(Default)]
pub struct MainOutputs {
    pub robot_to_ground: MainOutput<Option<Isometry3<Robot, Ground>>>,
    pub ground_to_robot: MainOutput<Option<Isometry3<Ground, Robot>>>,
}

impl GroundProvider {
    pub fn new(_context: CreationContext) -> Result<Self> {
        Ok(Self {})
    }

    pub fn cycle(&mut self, context: CycleContext) -> Result<MainOutputs> {
        struct ImuAdjustedRobot;

        let imu_roll_pitch = context.sensor_data.inertial_measurement_unit.roll_pitch;
        let imu_roll = imu_roll_pitch.x;
        let imu_pitch = imu_roll_pitch.y;

        let left_sole_to_robot = context.robot_kinematics.left_sole_to_robot;
        let imu_adjusted_robot_to_left_sole =
            Isometry3::<ImuAdjustedRobot, LeftSole>::rotation(Vector3::y_axis() * imu_pitch)
                * Isometry3::rotation(Vector3::x_axis() * imu_roll)
                * Isometry3::from(-left_sole_to_robot.origin());

        let right_sole_to_robot = context.robot_kinematics.right_sole_to_robot;
        let imu_adjusted_robot_to_right_sole =
            Isometry3::<ImuAdjustedRobot, RightSole>::rotation(Vector3::y_axis() * imu_pitch)
                * Isometry3::rotation(Vector3::x_axis() * imu_roll)
                * Isometry3::from(-right_sole_to_robot.origin());

        let left_sole_to_right_sole =
            right_sole_to_robot.origin().coords() - left_sole_to_robot.origin().coords();
        let left_sole_to_ground = vector![
            left_sole_to_right_sole.x(),
            left_sole_to_right_sole.y(),
            0.0
        ] * 0.5;

        let right_sole_to_ground = Isometry3::from(-left_sole_to_ground);
        let left_sole_to_ground = Isometry3::from(left_sole_to_ground);

        let robot_to_ground = context.support_foot.support_side.map(|side| match side {
            Side::Left => left_sole_to_ground * imu_adjusted_robot_to_left_sole,
            Side::Right => right_sole_to_ground * imu_adjusted_robot_to_right_sole,
        });
        Ok(MainOutputs {
            robot_to_ground: robot_to_ground.into(),
            ground_to_robot: robot_to_ground.map(|isometry| isometry.inverse()).into(),
        })
    }
}
