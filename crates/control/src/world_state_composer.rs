use context_attribute::context;
use framework::{MainOutput, OptionalInput, Parameter, RequiredInput};

pub struct WorldStateComposer {}

#[context]
pub struct NewContext {
    pub player_number: Parameter<PlayerNumber, "player_number">,
}

#[context]
pub struct CycleContext {
    pub ball_position: OptionalInput<BallPosition, "ball_position">,
    pub filtered_game_state: OptionalInput<FilteredGameState, "filtered_game_state">,
    pub game_controller_state: OptionalInput<GameControllerState, "game_controller_state">,
    pub penalty_shot_direction: OptionalInput<PenaltyShotDirection, "penalty_shot_direction">,
    pub robot_to_field: OptionalInput<Isometry2<f32>, "robot_to_field">,
    pub team_ball: OptionalInput<BallPosition, "team_ball">,

    pub player_number: Parameter<PlayerNumber, "player_number">,

    pub fall_state: RequiredInput<FallState, "fall_state">,
    pub has_ground_contact: RequiredInput<bool, "has_ground_contact">,
    pub obstacles: RequiredInput<Vec<Obstacle>, "obstacles">,
    pub primary_state: RequiredInput<PrimaryState, "primary_state">,
    pub role: RequiredInput<Role, "role">,
}

#[context]
#[derive(Default)]
pub struct MainOutputs {
    pub world_state: MainOutput<WorldState>,
}

impl WorldStateComposer {
    pub fn new(context: NewContext) -> anyhow::Result<Self> {
        Ok(Self {})
    }

    pub fn cycle(&mut self, context: CycleContext) -> anyhow::Result<MainOutputs> {
        Ok(MainOutputs::default())
    }
}
