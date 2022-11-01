use context_attribute::context;
use framework::{MainOutput, OptionalInput, Parameter};
use nalgebra::Isometry3;
use types::{CameraMatrices, HeadJoints, MotionCommand, RobotKinematics, SensorData};

pub struct LookAt {}

#[context]
pub struct NewContext {
    pub minimum_bottom_focus_pitch: Parameter<f32, "control/look_at/minimum_bottom_focus_pitch">,
}

#[context]
pub struct CycleContext {
    pub camera_matrices: OptionalInput<CameraMatrices, "camera_matrices?">,
    pub ground_to_robot: OptionalInput<Isometry3<f32>, "ground_to_robot?">,
    pub motion_command: OptionalInput<MotionCommand, "motion_command?">,
    pub robot_kinematics: OptionalInput<RobotKinematics, "robot_kinematics?">,
    pub sensor_data: OptionalInput<SensorData, "sensor_data?">,

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
