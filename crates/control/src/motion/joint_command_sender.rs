use context_attribute::context;
use framework::{MainOutput, Parameter, RequiredInput};
use types::{
    BodyJointsCommand, HeadJoints, HeadJointsCommand, Joints, JointsCommand, MotionSelection,
    SensorData,
};

pub struct JointCommandSender {}

#[context]
pub struct NewContext {
    pub center_head_position: Parameter<HeadJoints, "control/center_head_position">,
    pub penalized_pose: Parameter<Joints, "control/penalized_pose">,
    pub ready_pose: Parameter<Joints, "control/ready_pose">,
}

#[context]
pub struct CycleContext {
    pub center_head_position: Parameter<HeadJoints, "control/center_head_position">,
    pub penalized_pose: Parameter<Joints, "control/penalized_pose">,
    pub ready_pose: Parameter<Joints, "control/ready_pose">,

    pub arms_up_squat_joints_command: RequiredInput<JointsCommand, "arms_up_squat_joints_command">,
    pub dispatching_command: RequiredInput<JointsCommand, "dispatching_command">,
    pub fall_protection_command: RequiredInput<JointsCommand, "fall_protection_command">,
    pub head_joints_command: RequiredInput<HeadJointsCommand, "head_joints_command">,
    pub jump_left_joints_command: RequiredInput<JointsCommand, "jump_left_joints_command">,
    pub jump_right_joints_command: RequiredInput<JointsCommand, "jump_right_joints_command">,
    pub motion_selection: RequiredInput<MotionSelection, "motion_selection">,
    pub sensor_data: RequiredInput<SensorData, "sensor_data">,
    pub sit_down_joints_command: RequiredInput<JointsCommand, "sit_down_joints_command">,
    pub stand_up_back_positions: RequiredInput<Joints, "stand_up_back_positions">,
    pub stand_up_front_positions: RequiredInput<Joints, "stand_up_front_positions">,
    pub walk_joints_command: RequiredInput<BodyJointsCommand, "walk_joints_command">,
}

#[context]
#[derive(Default)]
pub struct MainOutputs {
    pub positions: MainOutput<Joints>,
    pub stiffnesses: MainOutput<Joints>,
}

impl JointCommandSender {
    pub fn new(_context: NewContext) -> anyhow::Result<Self> {
        Ok(Self {})
    }

    pub fn cycle(&mut self, _context: CycleContext) -> anyhow::Result<MainOutputs> {
        Ok(MainOutputs::default())
    }
}
