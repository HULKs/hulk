use color_eyre::Result;
use context_attribute::context;
use framework::MainOutput;
use nalgebra::{vector, Isometry3, Translation, Vector3};
use types::{RobotKinematics, SensorData, Side, SupportFoot};

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
    pub robot_to_ground: MainOutput<Option<Isometry3<f32>>>,
    pub ground_to_robot: MainOutput<Option<Isometry3<f32>>>,
}

impl GroundProvider {
    pub fn new(_context: CreationContext) -> Result<Self> {
        Ok(Self {})
    }

    pub fn cycle(&mut self, context: CycleContext) -> Result<MainOutputs> {
        let imu_roll_pitch = context.sensor_data.inertial_measurement_unit.roll_pitch;
        let imu_roll = imu_roll_pitch.x;
        let imu_pitch = imu_roll_pitch.y;

        let left_sole_to_robot = context.robot_kinematics.left_sole_to_robot;
        let imu_adjusted_robot_to_left_sole = Isometry3::rotation(Vector3::y() * imu_pitch)
            * Isometry3::rotation(Vector3::x() * imu_roll)
            * Isometry3::from(left_sole_to_robot.translation.inverse());

        let right_sole_to_robot = context.robot_kinematics.right_sole_to_robot;
        let imu_adjusted_robot_to_right_sole = Isometry3::rotation(Vector3::y() * imu_pitch)
            * Isometry3::rotation(Vector3::x() * imu_roll)
            * Isometry3::from(right_sole_to_robot.translation.inverse());

        let left_sole_to_right_sole =
            right_sole_to_robot.translation.vector - left_sole_to_robot.translation.vector;
        let left_sole_to_ground =
            0.5 * vector![left_sole_to_right_sole.x, left_sole_to_right_sole.y, 0.0];

        let robot_to_ground = context.support_foot.support_side.map(|side| match side {
            Side::Left => Translation::from(-left_sole_to_ground) * imu_adjusted_robot_to_left_sole,
            Side::Right => {
                Translation::from(left_sole_to_ground) * imu_adjusted_robot_to_right_sole
            }
        });
        Ok(MainOutputs {
            robot_to_ground: robot_to_ground.into(),
            ground_to_robot: robot_to_ground.map(|isometry| isometry.inverse()).into(),
        })
    }
}
