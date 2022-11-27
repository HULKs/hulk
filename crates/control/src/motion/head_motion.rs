use color_eyre::Result;
use context_attribute::context;
use framework::MainOutput;
use types::{HeadJoints, HeadJointsCommand, MotionCommand, SensorData};

pub struct HeadMotion {}

#[context]
pub struct NewContext {
    pub center_head_position: Parameter<HeadJoints, "control/center_head_position">,
    pub inner_maximum_pitch: Parameter<f32, "control/head_motion/inner_maximum_pitch">,
    pub maximum_velocity: Parameter<HeadJoints, "control/head_motion/maximum_velocity">,
    pub outer_maximum_pitch: Parameter<f32, "control/head_motion/outer_maximum_pitch">,
    pub outer_yaw: Parameter<f32, "control/head_motion/outer_yaw">,
}

#[context]
pub struct CycleContext {
    pub center_head_position: Parameter<HeadJoints, "control/center_head_position">,
    pub inner_maximum_pitch: Parameter<f32, "control/head_motion/inner_maximum_pitch">,
    pub maximum_velocity: Parameter<HeadJoints, "control/head_motion/maximum_velocity">,
    pub outer_maximum_pitch: Parameter<f32, "control/head_motion/outer_maximum_pitch">,
    pub outer_yaw: Parameter<f32, "control/head_motion/outer_yaw">,

    pub look_around: RequiredInput<Option<HeadJoints>, "look_around?">,
    pub look_at: RequiredInput<Option<HeadJoints>, "look_at?">,
    pub motion_command: RequiredInput<Option<MotionCommand>, "motion_command?">,
    pub sensor_data: Input<SensorData, "sensor_data">,
}

#[context]
#[derive(Default)]
pub struct MainOutputs {
    pub head_joints_command: MainOutput<Option<HeadJointsCommand>>,
}

impl HeadMotion {
    pub fn new(_context: NewContext) -> Result<Self> {
        Ok(Self {})
    }

    pub fn cycle(&mut self, _context: CycleContext) -> Result<MainOutputs> {
        Ok(MainOutputs::default())
    }
}
