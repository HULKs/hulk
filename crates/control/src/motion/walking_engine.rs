use context_attribute::context;
use framework::{AdditionalOutput, MainOutput, Parameter, PersistentState, RequiredInput};
use types::{
    configuration::{KickSteps, WalkingEngine as WalkingEngineConfiguration},
    BodyJointsCommand, Joints, MotionCommand, MotionSafeExits, RobotKinematics, SensorData, Step,
    StepAdjustment, SupportFoot, WalkCommand,
};

pub struct WalkingEngine {}

#[context]
pub struct NewContext {
    pub config: Parameter<WalkingEngineConfiguration, "control/walking_engine">,
    pub kick_steps: Parameter<KickSteps, "control/kick_steps">,
    pub ready_pose: Parameter<Joints, "control/ready_pose">,

    pub motion_safe_exits: PersistentState<MotionSafeExits, "motion_safe_exits">,
    pub walk_return_offset: PersistentState<Step, "walk_return_offset">,
}

#[context]
pub struct CycleContext {
    pub step_adjustment: AdditionalOutput<StepAdjustment, "step_adjustment">,
    pub walking_engine: AdditionalOutput<WalkingEngineConfiguration, "walking_engine">,

    pub config: Parameter<WalkingEngineConfiguration, "control/walking_engine">,
    pub kick_steps: Parameter<KickSteps, "control/kick_steps">,
    pub ready_pose: Parameter<Joints, "control/ready_pose">,

    pub motion_safe_exits: PersistentState<MotionSafeExits, "motion_safe_exits">,
    pub walk_return_offset: PersistentState<Step, "walk_return_offset">,

    pub motion_command: RequiredInput<MotionCommand, "motion_command">,
    pub robot_kinematics: RequiredInput<RobotKinematics, "robot_kinematics">,
    pub sensor_data: RequiredInput<SensorData, "sensor_data">,
    pub support_foot: RequiredInput<SupportFoot, "support_foot">,
    pub walk_command: RequiredInput<WalkCommand, "walk_command">,
}

#[context]
#[derive(Default)]
pub struct MainOutputs {
    pub walk_joints_command: MainOutput<BodyJointsCommand>,
}

impl WalkingEngine {
    pub fn new(_context: NewContext) -> anyhow::Result<Self> {
        Ok(Self {})
    }

    pub fn cycle(&mut self, _context: CycleContext) -> anyhow::Result<MainOutputs> {
        Ok(MainOutputs::default())
    }
}
