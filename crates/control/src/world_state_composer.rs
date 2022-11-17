use context_attribute::context;
use framework::MainOutput;
use nalgebra::Isometry2;
use spl_network_messages::PlayerNumber;
use types::{
    BallPosition, FallState, FilteredGameState, GameControllerState, Obstacle,
    PenaltyShotDirection, PrimaryState, Role, WorldState,
};

pub struct WorldStateComposer {}

#[context]
pub struct NewContext {
    pub player_number: Parameter<PlayerNumber, "player_number">,
}

#[context]
pub struct CycleContext {
    pub ball_position: RequiredInput<Option<BallPosition>, "ball_position?">,
    pub filtered_game_state: RequiredInput<Option<FilteredGameState>, "filtered_game_state?">,
    pub game_controller_state: RequiredInput<Option<GameControllerState>, "game_controller_state?">,
    pub penalty_shot_direction:
        RequiredInput<Option<PenaltyShotDirection>, "penalty_shot_direction?">,
    pub robot_to_field: RequiredInput<Option<Isometry2<f32>>, "robot_to_field?">,
    pub team_ball: RequiredInput<Option<BallPosition>, "team_ball?">,

    pub player_number: Parameter<PlayerNumber, "player_number">,

    pub fall_state: RequiredInput<Option<FallState>, "fall_state?">,
    pub has_ground_contact: Input<bool, "has_ground_contact">,
    pub obstacles: RequiredInput<Option<Vec<Obstacle>>, "obstacles?">,
    pub primary_state: RequiredInput<Option<PrimaryState>, "primary_state?">,
    pub role: RequiredInput<Option<Role>, "role?">,
}

#[context]
#[derive(Default)]
pub struct MainOutputs {
    pub world_state: MainOutput<Option<WorldState>>,
}

impl WorldStateComposer {
    pub fn new(_context: NewContext) -> anyhow::Result<Self> {
        Ok(Self {})
    }

    pub fn cycle(&mut self, _context: CycleContext) -> anyhow::Result<MainOutputs> {
        Ok(MainOutputs::default())
    }
}
