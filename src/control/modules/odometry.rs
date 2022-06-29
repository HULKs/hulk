use anyhow::Result;
use macros::{module, require_some};
use nalgebra::{Isometry2, Translation2, UnitComplex, Vector2};

use crate::types::{RobotKinematics, Side, SupportFoot};

pub struct Odometry {
    last_orientation: UnitComplex<f32>,
    last_left_sole_to_right_sole: Vector2<f32>,
    last_accumulated_odometry: Isometry2<f32>,
}

#[module(control)]
#[input(path = robot_kinematics, data_type = RobotKinematics)]
#[input(path = support_foot, data_type = SupportFoot)]
#[input(path = robot_orientation, data_type = UnitComplex<f32>)]
#[main_output(name = current_odometry_to_last_odometry, data_type = Isometry2<f32>)]
#[additional_output(path = accumulated_odometry, data_type = Isometry2<f32>)]
impl Odometry {}

impl Odometry {
    fn new(_context: NewContext) -> anyhow::Result<Self> {
        Ok(Self {
            last_left_sole_to_right_sole: Vector2::zeros(),
            last_orientation: UnitComplex::default(),
            last_accumulated_odometry: Isometry2::identity(),
        })
    }

    fn cycle(&mut self, mut context: CycleContext) -> Result<MainOutputs> {
        let robot_kinematics = require_some!(context.robot_kinematics);
        let support_foot = require_some!(context.support_foot);
        let &robot_orientation = require_some!(context.robot_orientation);

        let left_sole_to_right_sole = (robot_kinematics.right_sole_to_robot.translation.vector
            - robot_kinematics.left_sole_to_robot.translation.vector)
            .xy();
        let offset_to_last_position = calculate_offset_to_last_position(
            support_foot,
            &left_sole_to_right_sole,
            &self.last_left_sole_to_right_sole,
        );
        self.last_left_sole_to_right_sole = left_sole_to_right_sole;

        let orientation_offset = self.last_orientation.rotation_to(&robot_orientation);
        self.last_orientation = robot_orientation;

        let current_odometry_to_last_odometry = Isometry2::from_parts(
            Translation2::from(offset_to_last_position),
            orientation_offset,
        );
        let accumulated_odometry =
            current_odometry_to_last_odometry * self.last_accumulated_odometry;
        context
            .accumulated_odometry
            .fill_on_subscription(|| accumulated_odometry);
        self.last_accumulated_odometry = accumulated_odometry;

        Ok(MainOutputs {
            current_odometry_to_last_odometry: Some(current_odometry_to_last_odometry),
        })
    }
}

fn calculate_offset_to_last_position(
    support_foot: &SupportFoot,
    left_sole_to_right_sole: &Vector2<f32>,
    last_left_sole_to_right_sole: &Vector2<f32>,
) -> Vector2<f32> {
    match support_foot.support_side {
        Some(Side::Left) => (left_sole_to_right_sole - last_left_sole_to_right_sole) / 2.0,
        Some(Side::Right) => (-left_sole_to_right_sole + last_left_sole_to_right_sole) / 2.0,
        None => Vector2::zeros(),
    }
}
