use booster::FallDownState;
use color_eyre::Result;
use coordinate_systems::{Field, Ground};
use hsl_network_messages::PlayerNumber;
use linear_algebra::{Isometry2, Point2};
use serde::{Deserialize, Serialize};

use context_attribute::context;
use framework::{MainOutput, PerceptionInput};
use types::{
    ball_position::HypotheticalBallPosition,
    cycle_time::CycleTime,
    filtered_game_controller_state::FilteredGameControllerState,
    obstacles::Obstacle,
    primary_state::PrimaryState,
    rule_obstacles::RuleObstacle,
    world_state::{BallState, PlayerState, RobotState, WorldState},
};

#[derive(Deserialize, Serialize)]
pub struct WorldStateComposer {
    last_fall_down_state: Option<FallDownState>,
}

#[context]
pub struct CreationContext {}

#[context]
pub struct CycleContext {
    fall_down_state: PerceptionInput<Option<FallDownState>, "FallDownState", "fall_down_state?">,

    ball: Input<Option<BallState>, "ball_state?">,
    cycle_time: Input<CycleTime, "cycle_time">,
    filtered_game_controller_state:
        Input<Option<FilteredGameControllerState>, "filtered_game_controller_state?">,
    ground_to_field: Input<Option<Isometry2<Ground, Field>>, "ground_to_field?">,
    hypothetical_ball_position:
        Input<Vec<HypotheticalBallPosition<Ground>>, "hypothetical_ball_positions">,
    obstacles: Input<Vec<Obstacle>, "obstacles">,
    position_of_interest: Input<Point2<Ground>, "position_of_interest">,
    primary_state: Input<PrimaryState, "primary_state">,
    rule_ball: Input<Option<BallState>, "rule_ball_state?">,
    rule_obstacles: Input<Vec<RuleObstacle>, "rule_obstacles">,
    suggested_search_position: Input<Option<Point2<Field>>, "suggested_search_position?">,
    player_states: Input<Vec<PlayerState>, "player_states">,

    player_number: Parameter<PlayerNumber, "player_number">,
}

#[context]
#[derive(Default)]
pub struct MainOutputs {
    pub world_state: MainOutput<WorldState>,
}

impl WorldStateComposer {
    pub fn new(_context: CreationContext) -> Result<Self> {
        Ok(Self {
            last_fall_down_state: None,
        })
    }

    pub fn cycle(&mut self, context: CycleContext) -> Result<MainOutputs> {
        let fall_down_state = context
            .fall_down_state
            .persistent
            .into_iter()
            .chain(context.fall_down_state.temporary)
            .flat_map(|(_time, fall_down_states)| fall_down_states)
            .last()
            .flatten()
            .copied()
            .map_or(self.last_fall_down_state, |fall_down_state| {
                Some(fall_down_state)
            });

        if fall_down_state.is_some() {
            self.last_fall_down_state = fall_down_state;
        }

        let robot: RobotState = RobotState {
            ground_to_field: context.ground_to_field.copied(),
            player_number: *context.player_number,
            primary_state: *context.primary_state,
        };

        let world_state = WorldState {
            ball: context.ball.copied(),
            fall_down_state,
            filtered_game_controller_state: context.filtered_game_controller_state.cloned(),
            hypothetical_ball_positions: context.hypothetical_ball_position.clone(),
            now: context.cycle_time.start_time,
            obstacles: context.obstacles.clone(),
            player_states: context.player_states.clone(),
            position_of_interest: *context.position_of_interest,
            robot,
            rule_ball: context.rule_ball.copied(),
            rule_obstacles: context.rule_obstacles.clone(),
            suggested_search_position: context.suggested_search_position.copied(),
        };

        Ok(MainOutputs {
            world_state: world_state.into(),
        })
    }
}
