use module_derive::module;
use types::{
    BodyJointsCommand, HeadJoints, HeadJointsCommand, Joints, JointsCommand, MotionSelection,
    MotionType, SensorData,
};

pub struct JointCommandSender;

#[module(control)]
#[input(path = sensor_data, data_type = SensorData, required)]
#[input(path = motion_selection, data_type = MotionSelection, required)]
#[input(path = dispatching_command, data_type = JointsCommand, required)]
#[input(path = arms_up_squat_joints_command, data_type = JointsCommand, required)]
#[input(path = jump_left_joints_command, data_type = JointsCommand, required)]
#[input(path = jump_right_joints_command, data_type = JointsCommand, required)]
#[input(path = sit_down_joints_command, data_type = JointsCommand, required)]
#[input(path = stand_up_back_positions, data_type = Joints, required)]
#[input(path = stand_up_front_positions, data_type = Joints, required)]
#[input(path = walk_joints_command, data_type = BodyJointsCommand, required)]
#[input(path = head_joints_command, data_type = HeadJointsCommand, required)]
#[input(path = fall_protection_command, data_type = JointsCommand, required)]
#[parameter(path = control.penalized_pose, data_type = Joints)]
#[parameter(path = control.ready_pose, data_type = Joints)]
#[parameter(path = control.center_head_position, data_type = HeadJoints)]
#[main_output(name = positions, data_type = Joints)]
#[main_output(name = stiffnesses, data_type = Joints)]
impl JointCommandSender {}

impl JointCommandSender {
    fn new(_context: NewContext) -> anyhow::Result<Self> {
        Ok(Self)
    }

    fn cycle(&mut self, context: CycleContext) -> anyhow::Result<MainOutputs> {
        let current_positions = context.sensor_data.positions;
        let dispatching_command = context.dispatching_command;
        let fall_protection_positions = context.fall_protection_command.positions;
        let fall_protection_stiffnesses = context.fall_protection_command.stiffnesses;
        let head_joints_command = context.head_joints_command;
        let motion_selection = context.motion_selection;
        let arms_up_squat = context.arms_up_squat_joints_command;
        let jump_left = context.jump_left_joints_command;
        let jump_right = context.jump_right_joints_command;
        let sit_down = context.sit_down_joints_command;
        let stand_up_back_positions = context.stand_up_back_positions;
        let stand_up_front_positions = context.stand_up_front_positions;
        let walk = context.walk_joints_command;

        let (positions, stiffnesses) = match motion_selection.current_motion {
            MotionType::ArmsUpSquat => (arms_up_squat.positions, arms_up_squat.stiffnesses),
            MotionType::Dispatching => (
                dispatching_command.positions,
                dispatching_command.stiffnesses,
            ),
            MotionType::FallProtection => (fall_protection_positions, fall_protection_stiffnesses),
            MotionType::JumpLeft => (jump_left.positions, jump_left.stiffnesses),
            MotionType::JumpRight => (jump_right.positions, jump_right.stiffnesses),
            MotionType::Penalized => (*context.penalized_pose, Joints::fill(0.8)),
            MotionType::SitDown => (sit_down.positions, sit_down.stiffnesses),
            MotionType::Stand => (
                Joints::from_head_and_body(head_joints_command.positions, walk.positions),
                Joints::from_head_and_body(head_joints_command.stiffnesses, walk.stiffnesses),
            ),
            MotionType::StandUpBack => (*stand_up_back_positions, Joints::fill(1.0)),
            MotionType::StandUpFront => (*stand_up_front_positions, Joints::fill(1.0)),
            MotionType::Unstiff => (current_positions, Joints::fill(0.0)),
            MotionType::Walk => (
                Joints::from_head_and_body(head_joints_command.positions, walk.positions),
                Joints::from_head_and_body(head_joints_command.stiffnesses, walk.stiffnesses),
            ),
        };

        Ok(MainOutputs {
            positions: Some(positions),
            stiffnesses: Some(stiffnesses),
        })
    }
}
