use color_eyre::Result;
use coordinate_systems::{Field, Ground};
use hsl_network_messages::PlayerNumber;
use linear_algebra::{Isometry2, Point2};
use serde::{Deserialize, Serialize};

use context_attribute::context;
use framework::MainOutput;
use types::{
    ball_position::HypotheticalBallPosition,
    cycle_time::CycleTime,
    filtered_game_controller_state::FilteredGameControllerState,
    primary_state::PrimaryState,
    roles::Role,
    world_state::{BallState, RobotState, WorldState},
};

#[derive(Deserialize, Serialize)]
pub struct WorldStateComposer {}

#[context]
pub struct CreationContext {}

#[context]
pub struct CycleContext {
    ball: Input<Option<BallState>, "ball_state?">,
    cycle_time: Input<CycleTime, "cycle_time">,
    filtered_game_controller_state:
        Input<Option<FilteredGameControllerState>, "filtered_game_controller_state?">,
    ground_to_field: Input<Option<Isometry2<Ground, Field>>, "ground_to_field?">,
    hypothetical_ball_position:
        Input<Vec<HypotheticalBallPosition<Ground>>, "hypothetical_ball_positions">,
    // instant_kick_decisions: Input<Option<Vec<KickDecision>>, "instant_kick_decisions?">,
    // kick_decisions: Input<Option<Vec<KickDecision>>, "kick_decisions?">,
    // obstacles: Input<Vec<Obstacle>, "obstacles">,
    // position_of_interest: Input<Point2<Ground>, "position_of_interest">,
    primary_state: Input<PrimaryState, "primary_state">,
    // role: Input<Role, "role">,
    rule_ball: Input<Option<BallState>, "rule_ball_state?">,
    // rule_obstacles: Input<Vec<RuleObstacle>, "rule_obstacles">,
    suggested_search_position: Input<Option<Point2<Field>>, "suggested_search_position?">,

    player_number: Parameter<PlayerNumber, "player_number">,
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
        let robot: RobotState = RobotState {
            ground_to_field: context.ground_to_field.copied(),
            player_number: *context.player_number,
            primary_state: *context.primary_state,
            // role: *context.role,
            role: Role::Striker,
        };

        let world_state = WorldState {
            ball: context.ball.copied(),
            filtered_game_controller_state: context.filtered_game_controller_state.cloned(),
            hypothetical_ball_positions: context.hypothetical_ball_position.clone(),
            //instant_kick_decisions: context.instant_kick_decisions.cloned(),
            instant_kick_decisions: Default::default(),
            //kick_decisions: context.kick_decisions.cloned(),
            kick_decisions: Default::default(),
            now: context.cycle_time.start_time,
            //obstacles: context.obstacles.clone(),
            obstacles: Default::default(),
            // position_of_interest: *context.position_of_interest,
            position_of_interest: Point2::origin(),
            robot,
            rule_ball: context.rule_ball.copied(),
            // rule_obstacles: context.rule_obstacles.clone(),
            rule_obstacles: Default::default(),
            suggested_search_position: context.suggested_search_position.copied(),
        };

        Ok(MainOutputs {
            world_state: world_state.into(),
        })
    }
}
