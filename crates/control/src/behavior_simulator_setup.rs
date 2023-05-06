use std::time::{SystemTime, UNIX_EPOCH};

use color_eyre::Result;
use context_attribute::context;
use framework::MainOutput;
use hardware::TimeInterface;
use nalgebra::Isometry2;
use spl_network_messages::HulkMessage;
use types::{
    BallPosition, CycleTime, FallState, FilteredGameState, GameControllerState, Obstacle,
    PenaltyShotDirection, PrimaryState,
};

pub struct BehaviorSimulatorSetup {
    last_cycle_start: SystemTime,
}

#[context]
pub struct CreationContext {}

#[context]
pub struct CycleContext {
    pub hardware_interface: HardwareInterface,
}

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
    pub team_ball: MainOutput<Option<BallPosition>>,
}

impl BehaviorSimulatorSetup {
    pub fn new(_context: CreationContext) -> Result<Self> {
        Ok(Self {
            last_cycle_start: UNIX_EPOCH,
        })
    }

    pub fn cycle(&mut self, context: CycleContext<impl TimeInterface>) -> Result<MainOutputs> {
        let start_time = context.hardware_interface.get_now();
        let cycle_time = CycleTime {
            start_time,
            last_cycle_duration: start_time
                .duration_since(self.last_cycle_start)
                .expect("Nao time has run backwards"),
        };
        self.last_cycle_start = start_time;

        Ok(MainOutputs {
            ball_position: None.into(),
            cycle_time: cycle_time.into(),
            fall_state: FallState::Upright.into(),
            filtered_game_state: None.into(),
            game_controller_state: None.into(),
            has_ground_contact: true.into(),
            hulk_messages: Vec::new().into(),
            obstacles: Vec::new().into(),
            penalty_shot_direction: None.into(),
            primary_state: PrimaryState::Unstiff.into(),
            robot_to_field: None.into(),
            team_ball: None.into(),
        })
    }
}
