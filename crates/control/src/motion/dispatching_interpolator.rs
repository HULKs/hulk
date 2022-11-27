use color_eyre::Result;
use context_attribute::context;
use framework::MainOutput;
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
    pub arms_up_squat_joints_command:
        RequiredInput<Option<JointsCommand>, "arms_up_squat_joints_command?">,
    pub jump_left_joints_command: RequiredInput<Option<JointsCommand>, "jump_left_joints_command?">,
    pub jump_right_joints_command:
        RequiredInput<Option<JointsCommand>, "jump_right_joints_command?">,
    pub motion_selection: RequiredInput<Option<MotionSelection>, "motion_selection?">,
    pub sensor_data: Input<SensorData, "sensor_data">,
    pub sit_down_joints_command: RequiredInput<Option<JointsCommand>, "sit_down_joints_command?">,
    pub stand_up_back_positions: RequiredInput<Option<Joints>, "stand_up_back_positions?">,
    pub stand_up_front_positions: RequiredInput<Option<Joints>, "stand_up_front_positions?">,
    pub walk_joints_command: RequiredInput<Option<BodyJointsCommand>, "walk_joints_command?">,

    pub penalized_pose: Parameter<Joints, "control/penalized_pose">,
    pub ready_pose: Parameter<Joints, "control/ready_pose">,

    pub motion_safe_exits: PersistentState<MotionSafeExits, "motion_safe_exits">,
}

#[context]
#[derive(Default)]
pub struct MainOutputs {
    pub dispatching_command: MainOutput<Option<JointsCommand>>,
}

impl DispatchingInterpolator {
    pub fn new(_context: NewContext) -> Result<Self> {
        Ok(Self {})
    }

    pub fn cycle(&mut self, _context: CycleContext) -> Result<MainOutputs> {
        Ok(MainOutputs::default())
    }
}
