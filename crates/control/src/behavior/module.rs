use context_attribute::context;
use framework::{AdditionalOutput, MainOutput};
use nalgebra::Point2;
use types::{
    configuration::{Behavior as BehaviorConfiguration, LostBall},
    CameraMatrices, FieldDimensions, KickDecision, MotionCommand, PathObstacle, ProjectedLimbs,
    WorldState,
};

pub struct Behavior {}

#[context]
pub struct NewContext {
    pub behavior: Parameter<BehaviorConfiguration, "control/behavior">,
    pub field_dimensions: Parameter<FieldDimensions, "field_dimensions">,
    pub lost_ball_parameters: Parameter<LostBall, "control/behavior/lost_ball">,
}

#[context]
pub struct CycleContext {
    pub kick_decisions: AdditionalOutput<Vec<KickDecision>, "kick_decisions">,
    pub kick_targets: AdditionalOutput<Vec<Point2<f32>>, "kick_targets">,
    pub path_obstacles: AdditionalOutput<Vec<PathObstacle>, "path_obstacles">,

    pub camera_matrices: RequiredInput<Option<CameraMatrices>, "camera_matrices?">,
    pub projected_limbs: RequiredInput<Option<ProjectedLimbs>, "projected_limbs?">,

    pub behavior: Parameter<BehaviorConfiguration, "control/behavior">,
    pub field_dimensions: Parameter<FieldDimensions, "field_dimensions">,
    pub lost_ball_parameters: Parameter<LostBall, "control/behavior/lost_ball">,

    pub world_state: RequiredInput<Option<WorldState>, "world_state?">,
}

#[context]
#[derive(Default)]
pub struct MainOutputs {
    pub motion_command: MainOutput<Option<MotionCommand>>,
}

impl Behavior {
    pub fn new(_context: NewContext) -> anyhow::Result<Self> {
        Ok(Self {})
    }

    pub fn cycle(&mut self, _context: CycleContext) -> anyhow::Result<MainOutputs> {
        Ok(MainOutputs::default())
    }
}
