use color_eyre::Result;
use context_attribute::context;
use framework::MainOutput;
use projection::camera_matrix::CameraMatrix;
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize)]
pub struct CameraMatrixExtractor {}

#[context]
pub struct CreationContext {}

#[context]
pub struct CycleContext {
    camera_matrix: RequiredInput<Option<CameraMatrix>, "Control", "camera_matrix?">,
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
        Ok(MainOutputs {
            camera_matrix: Some(context.camera_matrix.clone()).into(),
        })
    }
}
