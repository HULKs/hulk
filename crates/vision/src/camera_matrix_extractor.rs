use color_eyre::Result;
use context_attribute::context;
use framework::MainOutput;
use types::{CameraMatrices, CameraMatrix, CameraPosition};

pub struct CameraMatrixExtractor {}

#[context]
pub struct CreationContext {}

#[context]
pub struct CycleContext {
    pub camera_matrices: RequiredInput<Option<CameraMatrices>, "Control", "camera_matrices?">,
    pub camera_position:
        Parameter<CameraPosition, "image_receiver.$cycler_instance.camera_position">,
}

#[context]
#[derive(Default)]
pub struct MainOutputs {
    pub camera_matrix: MainOutput<Option<CameraMatrix>>,
}

impl CameraMatrixExtractor {
    pub fn new(_context: CreationContext) -> Result<Self> {
        Ok(Self {})
    }

    pub fn cycle(&mut self, context: CycleContext) -> Result<MainOutputs> {
        let camera_matrix = match context.camera_position {
            CameraPosition::Top => &context.camera_matrices.top,
            CameraPosition::Bottom => &context.camera_matrices.bottom,
        };

        Ok(MainOutputs {
            camera_matrix: Some(camera_matrix.clone()).into(),
        })
    }
}
