use context_attribute::context;
use nalgebra::Isometry3;

use framework::{AdditionalOutput, MainOutput, Parameter, RequiredInput};
use types::{CameraMatrices, FieldDimensions, ProjectedFieldLines, RobotKinematics};

pub struct CameraMatrixProvider {}

#[context]
pub struct NewContext {}

#[context]
pub struct CycleContext {
    pub field_dimensions: Parameter<FieldDimensions, "field_dimensions">, // path?
    pub top_camera_matrix_parameters:
        Parameter<CameraMatrixParameters, "vision_top/camera_matrix_parameters">, // Kommt aus framework::configuration, path?
    pub bottom_camera_matrix_parameters:
        Parameter<CameraMatrixParameters, "vision_bottom/camera_matrix_parameters">, // Kommt aus framework::configuration, path?

    pub robot_to_ground: RequiredInput<Isometry3<f32>, "robot_to_ground">,
    pub robot_kinematics: RequiredInput<RobotKinematics, "robot_kinematics">,

    pub projected_field_lines: AdditionalOutput<ProjectedFieldLines, "projected_field_lines">,
}

#[context]
#[derive(Default)]
pub struct MainOutputs {
    pub camera_matrices: MainOutput<CameraMatrices>,
}

impl CameraMatrixProvider {
    pub fn new(context: NewContext) -> anyhow::Result<Self> {
        Ok(Self)
    }

    pub fn cycle(&mut self, context: CycleContext) -> anyhow::Result<MainOutputs> {
        Ok(MainOutputs::default())
    }
}
