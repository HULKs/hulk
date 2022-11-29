use color_eyre::Result;
use context_attribute::context;
use framework::MainOutput;
use types::{hardware::Image, CameraMatrices, CameraMatrix};

pub struct CameraMatrixExtractor {}

#[context]
pub struct NewContext {}

#[context]
pub struct CycleContext {
    pub camera_matrices: RequiredInput<Option<CameraMatrices>, "Control", "camera_matrices?">,
    pub image: Input<Image, "image">, // required for correct module order
}

#[context]
#[derive(Default)]
pub struct MainOutputs {
    pub camera_matrix: MainOutput<Option<CameraMatrix>>,
}

impl CameraMatrixExtractor {
    pub fn new(_context: NewContext) -> Result<Self> {
        Ok(Self {})
    }

    pub fn cycle(&mut self, _context: CycleContext) -> Result<MainOutputs> {
        Ok(MainOutputs::default())
    }
}
