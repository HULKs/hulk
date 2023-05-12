use color_eyre::Result;
use context_attribute::context;
use framework::MainOutput;
use nalgebra::Isometry2;
use spl_network_messages::HulkMessage;
use types::{
    BallPosition, CycleTime, FallState, FilteredGameState, GameControllerState, Obstacle,
    PenaltyShotDirection, PrimaryState,
};

pub struct FakeData {}

#[context]
pub struct CreationContext {}

#[context]
pub struct CycleContext {}

#[context]
#[derive(Default)]
pub struct MainOutputs {
    pub ball_position: MainOutput<Option<BallPosition>>,
    pub cycle_time: MainOutput<CycleTime>,
    pub fall_state: MainOutput<FallState>,
    pub filtered_game_state: MainOutput<Option<FilteredGameState>>,
    pub game_controller_state: MainOutput<Option<GameControllerState>>,
    pub has_ground_contact: MainOutput<bool>,
    pub hulk_messages: MainOutput<Vec<HulkMessage>>,
    pub obstacles: MainOutput<Vec<Obstacle>>,
    pub penalty_shot_direction: MainOutput<Option<PenaltyShotDirection>>,
    pub primary_state: MainOutput<PrimaryState>,
    pub robot_to_field: MainOutput<Option<Isometry2<f32>>>,
}

impl FakeData {
    pub fn new(_context: CreationContext) -> Result<Self> {
        Ok(Self {})
    }

    pub fn cycle(&mut self, _context: CycleContext) -> Result<MainOutputs> {
        Ok(MainOutputs::default())
    }
}
