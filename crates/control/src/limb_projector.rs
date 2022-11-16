use context_attribute::context;
use framework::{Input, MainOutput, Parameter, RequiredInput};
use nalgebra::Point3;
use types::{CameraMatrices, ProjectedLimbs, RobotKinematics};

pub struct LimbProjector {}

#[context]
pub struct NewContext {
    pub foot_bounding_polygon:
        Parameter<Vec<Point3<f32>>, "control/projected_limbs/foot_bounding_polygon">,
    pub knee_bounding_polygon:
        Parameter<Vec<Point3<f32>>, "control/projected_limbs/knee_bounding_polygon">,
    pub lower_arm_bounding_polygon:
        Parameter<Vec<Point3<f32>>, "control/projected_limbs/lower_arm_bounding_polygon">,
    pub torso_bounding_polygon:
        Parameter<Vec<Point3<f32>>, "control/projected_limbs/torso_bounding_polygon">,
    pub upper_arm_bounding_polygon:
        Parameter<Vec<Point3<f32>>, "control/projected_limbs/upper_arm_bounding_polygon">,
}

#[context]
pub struct CycleContext {
    pub camera_matrices: RequiredInput<Option<CameraMatrices>, "camera_matrices?">,
    pub robot_kinematics: Input<RobotKinematics, "robot_kinematics">,

    pub foot_bounding_polygon:
        Parameter<Vec<Point3<f32>>, "control/projected_limbs/foot_bounding_polygon">,
    pub knee_bounding_polygon:
        Parameter<Vec<Point3<f32>>, "control/projected_limbs/knee_bounding_polygon">,
    pub lower_arm_bounding_polygon:
        Parameter<Vec<Point3<f32>>, "control/projected_limbs/lower_arm_bounding_polygon">,
    pub torso_bounding_polygon:
        Parameter<Vec<Point3<f32>>, "control/projected_limbs/torso_bounding_polygon">,
    pub upper_arm_bounding_polygon:
        Parameter<Vec<Point3<f32>>, "control/projected_limbs/upper_arm_bounding_polygon">,
}

#[context]
#[derive(Default)]
pub struct MainOutputs {
    pub projected_limbs: MainOutput<Option<ProjectedLimbs>>,
}

impl LimbProjector {
    pub fn new(_context: NewContext) -> anyhow::Result<Self> {
        Ok(Self {})
    }

    pub fn cycle(&mut self, _context: CycleContext) -> anyhow::Result<MainOutputs> {
        Ok(MainOutputs::default())
    }
}
