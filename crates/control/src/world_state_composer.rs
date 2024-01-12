use color_eyre::Result;
use context_attribute::context;
use framework::MainOutput;
use nalgebra::{Isometry2, Point2};
use serde::{Deserialize, Serialize};
use spl_network_messages::PlayerNumber;
use types::{
    fall_state::FallState,
    filtered_game_controller_state::FilteredGameControllerState,
    game_controller_state::GameControllerState,
    kick_decision::KickDecision,
    obstacles::Obstacle,
    primary_state::PrimaryState,
    roles::Role,
    rule_obstacles::RuleObstacle,
    world_state::{BallState, RobotState, WorldState},
};

#[derive(Deserialize, Serialize)]
pub struct WorldStateComposer {}

#[context]
pub struct CreationContext {}

#[context]
pub struct CycleContext {
    ball: Input<Option<BallState>, "ball_state?">,
    rule_ball: Input<Option<BallState>, "rule_ball_state?">,
    filtered_game_controller_state:
        Input<Option<FilteredGameControllerState>, "filtered_game_controller_state?">,
    game_controller_state: Input<Option<GameControllerState>, "game_controller_state?">,
    robot_to_field: Input<Option<Isometry2<f32>>, "robot_to_field?">,
    kick_decisions: Input<Option<Vec<KickDecision>>, "kick_decisions?">,
    instant_kick_decisions: Input<Option<Vec<KickDecision>>, "instant_kick_decisions?">,

    player_number: Parameter<PlayerNumber, "player_number">,

    fall_state: Input<FallState, "fall_state">,
    has_ground_contact: Input<bool, "has_ground_contact">,
    obstacles: Input<Vec<Obstacle>, "obstacles">,
    rule_obstacles: Input<Vec<RuleObstacle>, "rule_obstacles">,
    primary_state: Input<PrimaryState, "primary_state">,
    role: Input<Role, "role">,
    position_of_interest: Input<Point2<f32>, "position_of_interest">,
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
            rule_ball: context.rule_ball.copied(),
            filtered_game_state: context
                .filtered_game_controller_state
                .map(|filtered_game_controller_state| filtered_game_controller_state.game_state),
            obstacles: context.obstacles.clone(),
            rule_obstacles: context.rule_obstacles.clone(),
            position_of_interest: *context.position_of_interest,
            robot,
            kick_decisions: context.kick_decisions.cloned(),
            instant_kick_decisions: context.instant_kick_decisions.cloned(),
            game_controller_state: context.game_controller_state.copied(),
        };

        Ok(MainOutputs {
            world_state: world_state.into(),
        })
    }
}
