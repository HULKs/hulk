use color_eyre::Result;
use context_attribute::context;
use framework::MainOutput;
use types::{image::Image, CameraMatrices, CameraMatrix};

use super::CyclerInstance;

pub struct CameraMatrixExtractor {}

#[context]
pub struct CreationContext {}

#[context]
pub struct CycleContext {
    pub camera_matrices: RequiredInput<Option<CameraMatrices>, "Control", "camera_matrices?">,
    pub image: Input<Image, "image">, // required for correct node order
    pub instance: CyclerInstance,
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
        let camera_matrix = match context.instance {
            CyclerInstance::VisionTop => &context.camera_matrices.top,
            CyclerInstance::VisionBottom => &context.camera_matrices.bottom,
        };

        Ok(MainOutputs {
            camera_matrix: Some(camera_matrix.clone()).into(),
        })
    }
}
