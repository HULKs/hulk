use color_eyre::Result;
use context_attribute::context;
use framework::{AdditionalOutput, MainOutput};
use types::{Ball, CameraMatrix, CandidateEvaluation, PerspectiveGridCandidates};

pub struct BallDetection {}

#[context]
pub struct CreationContext {}

#[context]
pub struct CycleContext {
    pub ball_candidates: AdditionalOutput<Vec<CandidateEvaluation>, "ball_candidates">,

    pub camera_matrix: RequiredInput<Option<CameraMatrix>, "camera_matrix?">,
    pub perspective_grid_candidates:
        RequiredInput<Option<PerspectiveGridCandidates>, "perspective_grid_candidates?">,
}

#[context]
#[derive(Default)]
pub struct MainOutputs {
    pub balls: MainOutput<Option<Vec<Ball>>>,
}

impl BallDetection {
    pub fn new(_context: CreationContext) -> Result<Self> {
        Ok(Self {})
    }

    pub fn cycle(&mut self, _context: CycleContext) -> Result<MainOutputs> {
        Ok(MainOutputs::default())
    }
}
