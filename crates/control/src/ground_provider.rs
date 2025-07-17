use color_eyre::Result;
use serde::{Deserialize, Serialize};

use context_attribute::context;
use coordinate_systems::{Field, Ground, Robot};
use framework::MainOutput;
use linear_algebra::{vector, Isometry3, Orientation3};
use types::{robot_kinematics::RobotKinematics, support_foot::Side};

#[derive(Deserialize, Serialize)]
pub struct GroundProvider {}

#[context]
pub struct CreationContext {}

#[context]
pub struct CycleContext {
    robot_kinematics: Input<RobotKinematics, "robot_kinematics">,
    robot_orientation: RequiredInput<Option<Orientation3<Field>>, "robot_orientation?">,
    support_side: RequiredInput<Option<Side>, "support_foot.support_side?">,
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
        struct LeftSoleHorizontal;
        struct RightSoleHorizontal;

        let (roll, pitch, _) = context.robot_orientation.inner.euler_angles();
        let imu_orientation = Orientation3::from_euler_angles(roll, pitch, 0.0).mirror();

        let left_sole_horizontal_to_robot = Isometry3::from_parts(
            context
                .robot_kinematics
                .left_leg
                .sole_to_robot
                .translation()
                .coords(),
            imu_orientation,
        );
        let right_sole_horizontal_to_robot = Isometry3::from_parts(
            context
                .robot_kinematics
                .right_leg
                .sole_to_robot
                .translation()
                .coords(),
            imu_orientation,
        );

        let left_sole_in_robot = context
            .robot_kinematics
            .left_leg
            .sole_to_robot
            .translation();
        let right_sole_in_robot = context
            .robot_kinematics
            .right_leg
            .sole_to_robot
            .translation();

        let left_sole_to_right_sole = right_sole_in_robot - left_sole_in_robot;
        let ground_to_left_sole = Isometry3::<Ground, LeftSoleHorizontal>::from(
            vector![
                left_sole_to_right_sole.x(),
                left_sole_to_right_sole.y(),
                0.0
            ] / 2.0,
        );
        let ground_to_right_sole = Isometry3::<Ground, RightSoleHorizontal>::from(
            -vector![
                left_sole_to_right_sole.x(),
                left_sole_to_right_sole.y(),
                0.0
            ] / 2.0,
        );

        let ground_to_robot = match context.support_side {
            Side::Left => left_sole_horizontal_to_robot * ground_to_left_sole,
            Side::Right => right_sole_horizontal_to_robot * ground_to_right_sole, //ground_to_right_sole * robot_to_right_support_sole,
        };

        Ok(MainOutputs {
            robot_to_ground: Some(ground_to_robot.inverse()).into(),
            ground_to_robot: Some(ground_to_robot).into(),
        })
    }
}
