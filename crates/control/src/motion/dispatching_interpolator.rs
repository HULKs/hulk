use context_attribute::context;
use framework::{MainOutput, Input, Parameter, PersistentState};
use types::{
    BodyJointsCommand, Joints, JointsCommand, MotionSafeExits, MotionSelection, SensorData,
};

pub struct DispatchingInterpolator {}

#[context]
pub struct NewContext {
    pub penalized_pose: Parameter<Joints, "control/penalized_pose">,
    pub ready_pose: Parameter<Joints, "control/ready_pose">,

    pub motion_safe_exits: PersistentState<MotionSafeExits, "motion_safe_exits">,
}

#[context]
pub struct CycleContext {
    pub arms_up_squat_joints_command: Input<JointsCommand, "arms_up_squat_joints_command?">,
    pub jump_left_joints_command: Input<JointsCommand, "jump_left_joints_command?">,
    pub jump_right_joints_command: Input<JointsCommand, "jump_right_joints_command?">,
    pub motion_selection: Input<MotionSelection, "motion_selection?">,
    pub sensor_data: Input<SensorData, "sensor_data?">,
    pub sit_down_joints_command: Input<JointsCommand, "sit_down_joints_command?">,
    pub stand_up_back_positions: Input<Joints, "stand_up_back_positions?">,
    pub stand_up_front_positions: Input<Joints, "stand_up_front_positions?">,
    pub walk_joints_command: Input<BodyJointsCommand, "walk_joints_command?">,

    pub penalized_pose: Parameter<Joints, "control/penalized_pose">,
    pub ready_pose: Parameter<Joints, "control/ready_pose">,

    pub motion_safe_exits: PersistentState<MotionSafeExits, "motion_safe_exits">,
}

#[context]
#[derive(Default)]
pub struct MainOutputs {
    pub dispatching_command: MainOutput<JointsCommand>,
}

impl DispatchingInterpolator {
    pub fn new(_context: NewContext) -> anyhow::Result<Self> {
        Ok(Self {})
    }

    pub fn cycle(&mut self, _context: CycleContext) -> anyhow::Result<MainOutputs> {
        Ok(MainOutputs::default())
    }
}
