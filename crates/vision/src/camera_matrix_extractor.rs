use color_eyre::Result;
use context_attribute::context;
use framework::MainOutput;
use serde::{Deserialize, Serialize};
use types::{
    camera_matrix::{CameraMatrices, CameraMatrix},
    camera_position::CameraPosition,
};

#[derive(Deserialize, Serialize)]
pub struct CameraMatrixExtractor {}

#[context]
pub struct CreationContext {}

#[context]
pub struct CycleContext {
    camera_matrices: RequiredInput<Option<CameraMatrices>, "Control", "camera_matrices?">,
    camera_position: Parameter<CameraPosition, "image_receiver.$cycler_instance.camera_position">,
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
