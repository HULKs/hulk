use coordinate_systems::{Robot, Walk};
use kinematics::inverse::leg_angles;
use linear_algebra::{point, vector, Isometry3, Orientation3, Point3, Vector2, Vector3};
use serde::{Deserialize, Serialize};
use serialize_hierarchy::SerializeHierarchy;
use types::{
    joints::{
        body::{BodyJoints, UpperBodyJoints},
        mirror::Mirror,
    },
    step_plan::Step,
    support_foot::Side,
};

use super::{arms::Arm, CycleContext};

pub fn robot_to_walk(context: &CycleContext) -> Isometry3<Robot, Walk> {
    Isometry3::from_parts(
        vector![
            context.parameters.base.torso_offset,
            0.0,
            context.parameters.base.walk_height,
        ],
        Orientation3::new(Vector3::y_axis() * context.parameters.base.torso_tilt),
    )
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, SerializeHierarchy)]
pub struct Feet {
    pub support_foot: Point3<Walk>,
    pub swing_foot: Point3<Walk>,
    pub swing_turn: f32,
}

impl Feet {
    pub fn end_from_request(context: &CycleContext, step: Step, support_side: Side) -> Self {
        let (support_base_offset, swing_base_offset) = match support_side {
            Side::Left => (
                context.parameters.base.foot_offset_left,
                context.parameters.base.foot_offset_right,
            ),
            Side::Right => (
                context.parameters.base.foot_offset_right,
                context.parameters.base.foot_offset_left,
            ),
        };
        let support_foot = point![-step.forward / 2.0, -step.left / 2.0, 0.0] + support_base_offset;
        let swing_foot = point![step.forward / 2.0, step.left / 2.0, 0.0] + swing_base_offset;
        let swing_turn = step.turn / 2.0;
        Feet {
            support_foot,
            swing_foot,
            swing_turn,
        }
    }

    pub fn swing_travel_over_ground(&self, end: &Feet) -> Vector2<Walk> {
        ((self.support_foot - self.swing_foot) + (end.swing_foot - end.support_foot)).xy()
    }

    pub fn swap_sides(self) -> Self {
        Feet {
            support_foot: self.swing_foot,
            swing_foot: self.support_foot,
            swing_turn: -self.swing_turn,
        }
    }

    pub fn compute_joints(
        self,
        context: &CycleContext,
        support_side: Side,
        left_arm: &Arm,
        right_arm: &Arm,
    ) -> BodyJoints<f32> {
        let (left_foot, right_foot, left_turn, right_turn) = match support_side {
            Side::Left => (
                self.support_foot,
                self.swing_foot,
                -self.swing_turn,
                self.swing_turn,
            ),
            Side::Right => (
                self.swing_foot,
                self.support_foot,
                self.swing_turn,
                -self.swing_turn,
            ),
        };
        let walk_to_robot = robot_to_walk(context).inverse();

        let left_foot_to_robot = walk_to_robot
            * Isometry3::from_parts(
                left_foot.coords(),
                Orientation3::new(Vector3::z_axis() * left_turn),
            );
        let right_foot_to_robot = walk_to_robot
            * Isometry3::from_parts(
                right_foot.coords(),
                Orientation3::new(Vector3::z_axis() * right_turn),
            );

        let (_, leg_joints) = leg_angles(left_foot_to_robot, right_foot_to_robot);

        let arm_joints = UpperBodyJoints {
            left_arm: left_arm.compute_joints(context, leg_joints.left_leg, right_foot),
            right_arm: right_arm
                .compute_joints(
                    context,
                    leg_joints.right_leg.mirrored(),
                    point![left_foot.x(), -left_foot.y(), left_foot.z()],
                )
                .mirrored(),
        };

        BodyJoints::from_lower_and_upper(leg_joints, arm_joints)
    }
}

// visualized in desmos: https://www.desmos.com/calculator/kcr3uxqmyw
pub fn parabolic_return(x: f32, midpoint: f32) -> f32 {
    if x < midpoint {
        -2.0 / midpoint.powi(3) * x.powi(3) + 3.0 / midpoint.powi(2) * x.powi(2)
    } else {
        -1.0 / (midpoint - 1.0).powi(3)
            * (2.0 * x.powi(3) - 3.0 * (midpoint + 1.0) * x.powi(2) + 6.0 * midpoint * x
                - 3.0 * midpoint
                + 1.0)
    }
}

pub fn parabolic_step(x: f32) -> f32 {
    if x < 0.5 {
        2.0 * x * x
    } else {
        4.0 * x - 2.0 * x * x - 1.0
    }
}
