use context_attribute::context;
use framework::{MainOutput, Input, Parameter};
use nalgebra::Isometry3;
use types::{CameraMatrices, HeadJoints, MotionCommand, RobotKinematics, SensorData};

pub struct LookAt {}

#[context]
pub struct NewContext {
    pub minimum_bottom_focus_pitch: Parameter<f32, "control/look_at/minimum_bottom_focus_pitch">,
}

#[context]
pub struct CycleContext {
    pub camera_matrices: Input<CameraMatrices, "camera_matrices?">,
    pub ground_to_robot: Input<Isometry3<f32>, "ground_to_robot?">,
    pub motion_command: Input<Option<MotionCommand>, "motion_command?">,
    pub robot_kinematics: Input<RobotKinematics, "robot_kinematics?">,
    pub sensor_data: Input<SensorData, "sensor_data?">,

    pub minimum_bottom_focus_pitch: Parameter<f32, "control/look_at/minimum_bottom_focus_pitch">,
}

#[context]
#[derive(Default)]
pub struct MainOutputs {
    pub look_at: MainOutput<HeadJoints>,
}

impl LookAt {
    pub fn new(_context: NewContext) -> anyhow::Result<Self> {
        Ok(Self {})
    }

    pub fn cycle(&mut self, _context: CycleContext) -> anyhow::Result<MainOutputs> {
        Ok(MainOutputs::default())
    }
}
