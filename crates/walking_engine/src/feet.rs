use coordinate_systems::{Robot, Walk};
use kinematics::forward::{left_sole_to_robot, right_sole_to_robot};
use linear_algebra::{point, Isometry3, Orientation3, Pose3, Vector2, Vector3};
use path_serde::{PathDeserialize, PathIntrospect, PathSerialize};
use serde::{Deserialize, Serialize};
use types::{joints::body::BodyJoints, step::Step, support_foot::Side};

use crate::parameters::Parameters;

#[derive(
    Clone, Copy, Debug, Serialize, Deserialize, PathSerialize, PathDeserialize, PathIntrospect,
)]
pub struct Feet {
    pub support_sole: Pose3<Walk>,
    pub swing_sole: Pose3<Walk>,
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
