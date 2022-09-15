use framework::{
    MainOutput, AdditionalOutput, RequiredInput, OptionalInput, Parameter
};

pub struct Behavior {}

#[context]
pub struct NewContext {
    pub behavior: Parameter<configuration::Behavior, "control/behavior">,
    pub field_dimensions: Parameter<FieldDimensions, "field_dimensions">,
    pub lost_ball_parameters: Parameter<configuration::LostBall, "control/behavior/lost_ball">,
}

#[context]
pub struct CycleContext {
    pub kick_decisions: AdditionalOutput<Vec<KickDecision>, "kick_decisions">,
    pub kick_targets: AdditionalOutput<Vec<Point2<f32>>, "kick_targets">,
    pub path_obstacles: AdditionalOutput<Vec<PathObstacle>, "path_obstacles">,


    pub camera_matrices: OptionalInput<CameraMatrices, "camera_matrices">,
    pub projected_limbs: OptionalInput<ProjectedLimbs, "projected_limbs">,

    pub behavior: Parameter<configuration::Behavior, "control/behavior">,
    pub field_dimensions: Parameter<FieldDimensions, "field_dimensions">,
    pub lost_ball_parameters: Parameter<configuration::LostBall, "control/behavior/lost_ball">,



    pub world_state: RequiredInput<WorldState, "world_state">,
}

#[context]
#[derive(Default)]
pub struct MainOutputs {
    pub motion_command: MainOutput<MotionCommand>,
}

impl Behavior {
    pub fn new(context: NewContext) -> anyhow::Result<Self> {
        Ok(Self {})
    }

    pub fn cycle(&mut self, context: CycleContext) -> anyhow::Result<MainOutputs> {
        Ok(MainOutputs::default())
    }
}
