use types::{joints::body::LowerBodyJoints, support_foot::Side};

use crate::parameters::StiffnessLossCompensation;

pub trait CompensateStiffnessLossExt {
    fn compensate_stiffness_loss(
        self,
        parameters: &StiffnessLossCompensation,
        last_joint_commands: &LowerBodyJoints,
        measured_positions: &LowerBodyJoints,
        support_side: Side,
    ) -> Self;
}

impl CompensateStiffnessLossExt for LowerBodyJoints {
    fn compensate_stiffness_loss(
        mut self,
        parameters: &StiffnessLossCompensation,
        last_actuated_joints: &LowerBodyJoints,
        measured_joints: &LowerBodyJoints,
        support_side: Side,
    ) -> Self {
        let support_leg = match support_side {
            Side::Left => &mut self.left_leg,
            Side::Right => &mut self.right_leg,
        };
        let last_actuated_support_leg = match support_side {
            Side::Left => &last_actuated_joints.left_leg,
            Side::Right => &last_actuated_joints.right_leg,
        };
        let measured_support_leg = match support_side {
            Side::Left => &measured_joints.left_leg,
            Side::Right => &measured_joints.right_leg,
        };

        let ankle_pitch_stiffness_loss =
            measured_support_leg.ankle_pitch - last_actuated_support_leg.ankle_pitch;

        *support_leg += parameters.ankle_pitch * ankle_pitch_stiffness_loss;

        self
    }
}
