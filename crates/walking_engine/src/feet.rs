use coordinate_systems::{Robot, Walk};
use kinematics::forward::{left_sole_to_robot, right_sole_to_robot};
use linear_algebra::{point, Isometry3, Orientation3, Pose2, Pose3, Vector2, Vector3};
use path_serde::{PathDeserialize, PathIntrospect, PathSerialize};
use serde::{Deserialize, Serialize};
use types::{joints::body::BodyJoints, step::Step, support_foot::Side};

use crate::parameters::Parameters;

#[derive(
    Clone, Copy, Debug, Serialize, Deserialize, PathSerialize, PathDeserialize, PathIntrospect,
)]
pub struct Feet<T = Pose3<Walk>> {
    pub support_sole: T,
    pub swing_sole: T,
}

impl Feet<Pose2<Walk>> {
    pub fn at_ground(self) -> Feet<Pose3<Walk>> {
        let support_sole = Pose3::from_parts(
            self.support_sole.position().extend(0.0),
            Orientation3::from_euler_angles(0.0, 0.0, self.support_sole.orientation().angle()),
        );
        let swing_sole = Pose3::from_parts(
            self.swing_sole.position().extend(0.0),
            Orientation3::from_euler_angles(0.0, 0.0, self.swing_sole.orientation().angle()),
        );
        Feet {
            support_sole,
            swing_sole,
        }
    }
}

impl Feet {
    pub fn from_joints(
        robot_to_walk: Isometry3<Robot, Walk>,
        joints: &BodyJoints,
        support_side: Side,
    ) -> Self {
        let left_sole = robot_to_walk * left_sole_to_robot(&joints.left_leg).as_pose();
        let right_sole = robot_to_walk * right_sole_to_robot(&joints.right_leg).as_pose();

        match support_side {
            Side::Left => Feet {
                support_sole: left_sole,
                swing_sole: right_sole,
            },
            Side::Right => Feet {
                support_sole: right_sole,
                swing_sole: left_sole,
            },
        }
    }

    pub fn to_step(&self, parameters: &Parameters, support_side: Side) -> Step {
        let swing_base_offset = match support_side {
            Side::Left => parameters.base.foot_offset_right,
            Side::Right => parameters.base.foot_offset_left,
        };

        let swing_sole_x = self.swing_sole.position().x() - swing_base_offset.x();
        let swing_sole_y = self.swing_sole.position().y() - swing_base_offset.y();
        // let swing_sole_angle = self
        //     .swing_sole
        //     .orientation()
        //     .rotation()
        //     .inner
        //     .euler_angles()
        //     .2;

        let forward = 2. * swing_sole_x;
        let left = 2. * swing_sole_y;
        // let turn = 2. * swing_sole_angle;

        Step {
            forward,
            left,
            turn: 0.0,
        }
    }

    pub fn end_from_request(parameters: &Parameters, request: Step, support_side: Side) -> Self {
        let (support_base_offset, swing_base_offset) = match support_side {
            Side::Left => (
                parameters.base.foot_offset_left,
                parameters.base.foot_offset_right,
            ),
            Side::Right => (
                parameters.base.foot_offset_right,
                parameters.base.foot_offset_left,
            ),
        };
        let support_sole = Pose3::from_parts(
            point![-request.forward / 2.0, -request.left / 2.0, 0.0] + support_base_offset,
            Orientation3::new(Vector3::z_axis() * -request.turn / 2.0),
        );
        let swing_sole = Pose3::from_parts(
            point![request.forward / 2.0, request.left / 2.0, 0.0] + swing_base_offset,
            Orientation3::new(Vector3::z_axis() * request.turn / 2.0),
        );
        Feet {
            support_sole,
            swing_sole,
        }
    }

    pub fn swing_travel_over_ground(&self, end: &Feet) -> Vector2<Walk> {
        ((self.support_sole.position() - self.swing_sole.position())
            + (end.swing_sole.position() - end.support_sole.position()))
        .xy()
    }
}
