use context_attribute::context;
use framework::{AdditionalOutput, MainOutput, OptionalInput, Parameter};

pub struct CameraMatrixProvider {}

#[context]
pub struct NewContext {
    pub bottom_camera_matrix_parameters:
        Parameter<CameraMatrixParameters, "vision_bottom/camera_matrix_parameters">,
    pub field_dimensions: Parameter<FieldDimensions, "field_dimensions">,
    pub top_camera_matrix_parameters:
        Parameter<CameraMatrixParameters, "vision_top/camera_matrix_parameters">,
}

#[context]
pub struct CycleContext {
    pub projected_field_lines: AdditionalOutput<ProjectedFieldLines, "projected_field_lines">,

    pub robot_kinematics: OptionalInput<RobotKinematics, "robot_kinematics?">,
    pub robot_to_ground: OptionalInput<Isometry3<f32>, "robot_to_ground?">,

    pub bottom_camera_matrix_parameters:
        Parameter<CameraMatrixParameters, "vision_bottom/camera_matrix_parameters">,
    pub field_dimensions: Parameter<FieldDimensions, "field_dimensions">,
    pub top_camera_matrix_parameters:
        Parameter<CameraMatrixParameters, "vision_top/camera_matrix_parameters">,
}

#[context]
#[derive(Default)]
pub struct MainOutputs {
    pub camera_matrices: MainOutput<CameraMatrices>,
}

impl CameraMatrixProvider {
    pub fn new(context: NewContext) -> anyhow::Result<Self> {
        Ok(Self {})
    }

    pub fn cycle(&mut self, context: CycleContext) -> anyhow::Result<MainOutputs> {
        Ok(MainOutputs::default())
    }
}
