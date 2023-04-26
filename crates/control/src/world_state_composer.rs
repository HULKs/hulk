use color_eyre::Result;
use context_attribute::context;
use framework::MainOutput;
use nalgebra::{Isometry2, Point2};
use spl_network_messages::PlayerNumber;
use types::{
    BallState, FallState, FilteredGameState, GameControllerState, KickDecision, Obstacle,
    PenaltyShotDirection, PrimaryState, RobotState, Role, WorldState,
};

pub struct WorldStateComposer {}

#[context]
pub struct CreationContext {
    pub player_number: Parameter<PlayerNumber, "player_number">,
}

#[context]
pub struct CycleContext {
    pub ball: Input<Option<BallState>, "ball_state?">,
    pub filtered_game_state: Input<Option<FilteredGameState>, "filtered_game_state?">,
    pub game_controller_state: Input<Option<GameControllerState>, "game_controller_state?">,
    pub penalty_shot_direction: Input<Option<PenaltyShotDirection>, "penalty_shot_direction?">,
    pub robot_to_field: Input<Option<Isometry2<f32>>, "robot_to_field?">,
    pub kick_decisions: Input<Option<Vec<KickDecision>>, "kick_decisions?">,

    pub player_number: Parameter<PlayerNumber, "player_number">,

    pub fall_state: Input<FallState, "fall_state">,
    pub has_ground_contact: Input<bool, "has_ground_contact">,
    pub obstacles: Input<Vec<Obstacle>, "obstacles">,
    pub primary_state: Input<PrimaryState, "primary_state">,
    pub role: Input<Role, "role">,
    pub position_of_interest: Input<Point2<f32>, "position_of_interest">,
}

#[context]
#[derive(Default)]
pub struct MainOutputs {
    pub world_state: MainOutput<WorldState>,
}

impl WorldStateComposer {
    pub fn new(_context: CreationContext) -> Result<Self> {
        Ok(Self {})
    }

    pub fn cycle(&mut self, context: CycleContext) -> Result<MainOutputs> {
        let robot = RobotState {
            robot_to_field: context.robot_to_field.copied(),
            role: *context.role,
            primary_state: *context.primary_state,
            fall_state: *context.fall_state,
            has_ground_contact: *context.has_ground_contact,
            player_number: *context.player_number,
        };

        let world_state = WorldState {
            ball: context.ball.copied(),
            filtered_game_state: context.filtered_game_state.copied(),
            obstacles: context.obstacles.clone(),
            position_of_interest: *context.position_of_interest,
            robot,
            kick_decisions: context.kick_decisions.cloned(),
            game_controller_state: context.game_controller_state.copied(),
        };

        Ok(MainOutputs {
            world_state: world_state.into(),
        })
    }
}
