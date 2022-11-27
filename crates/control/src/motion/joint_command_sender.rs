use color_eyre::Result;
use context_attribute::context;
use framework::MainOutput;
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

    pub arms_up_squat_joints_command:
        RequiredInput<Option<JointsCommand>, "arms_up_squat_joints_command?">,
    pub dispatching_command: RequiredInput<Option<JointsCommand>, "dispatching_command?">,
    pub fall_protection_command: RequiredInput<Option<JointsCommand>, "fall_protection_command?">,
    pub head_joints_command: RequiredInput<Option<HeadJointsCommand>, "head_joints_command?">,
    pub jump_left_joints_command: RequiredInput<Option<JointsCommand>, "jump_left_joints_command?">,
    pub jump_right_joints_command:
        RequiredInput<Option<JointsCommand>, "jump_right_joints_command?">,
    pub motion_selection: RequiredInput<Option<MotionSelection>, "motion_selection?">,
    pub sensor_data: Input<SensorData, "sensor_data">,
    pub sit_down_joints_command: RequiredInput<Option<JointsCommand>, "sit_down_joints_command?">,
    pub stand_up_back_positions: RequiredInput<Option<Joints>, "stand_up_back_positions?">,
    pub stand_up_front_positions: RequiredInput<Option<Joints>, "stand_up_front_positions?">,
    pub walk_joints_command: RequiredInput<Option<BodyJointsCommand>, "walk_joints_command?">,
}

#[context]
#[derive(Default)]
pub struct MainOutputs {
    pub positions: MainOutput<Option<Joints>>,
    pub stiffnesses: MainOutput<Option<Joints>>,
}

impl JointCommandSender {
    pub fn new(_context: NewContext) -> Result<Self> {
        Ok(Self {})
    }

    pub fn cycle(&mut self, _context: CycleContext) -> Result<MainOutputs> {
        Ok(MainOutputs::default())
    }
}
