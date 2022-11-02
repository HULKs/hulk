use std::sync::Arc;

use context_attribute::context;
use framework::{MainOutput, OptionalInput};
use types::{CameraMatrices, CameraMatrix};

pub struct CameraMatrixExtractor {}

#[context]
pub struct NewContext {}

#[context]
pub struct CycleContext {
    pub camera_matrices: OptionalInput<CameraMatrices, "Control", "camera_matrices?">,
    pub image: RequiredInput<Arc<bool>, "image">, // required for correct module order
}

#[context]
#[derive(Default)]
pub struct MainOutputs {
    pub camera_matrix: MainOutput<CameraMatrix>,
}

impl CameraMatrixExtractor {
    pub fn new(_context: NewContext) -> anyhow::Result<Self> {
        Ok(Self {})
    }

    pub fn cycle(&mut self, _context: CycleContext) -> anyhow::Result<MainOutputs> {
        Ok(MainOutputs::default())
    }
}
