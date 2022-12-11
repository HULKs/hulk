use color_eyre::Result;
use context_attribute::context;
use framework::{AdditionalOutput, MainOutput};
use types::{
    configuration::BallDetection as BallDetectionConfiguration, Ball, CameraMatrix,
    CandidateEvaluation, PerspectiveGridCandidates,
};

pub struct BallDetection {}

#[context]
pub struct CreationContext {
    // TODO: remove these configuration structs and explicitely list parameters
    pub ball_detection: Parameter<BallDetectionConfiguration, "ball_detection.$cycler_instance">,
    pub ball_radius: Parameter<f32, "field_dimensions.ball_radius">,
}

#[context]
pub struct CycleContext {
    pub ball_candidates: AdditionalOutput<Vec<CandidateEvaluation>, "ball_candidates">,

    pub camera_matrix: RequiredInput<Option<CameraMatrix>, "camera_matrix?">,
    pub perspective_grid_candidates:
        RequiredInput<Option<PerspectiveGridCandidates>, "perspective_grid_candidates?">,

    pub ball_detection: Parameter<BallDetectionConfiguration, "ball_detection.$cycler_instance">,
    pub ball_radius: Parameter<f32, "field_dimensions.ball_radius">,
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
