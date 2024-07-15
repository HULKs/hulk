use color_eyre::Result;
use serde::{Deserialize, Serialize};

use context_attribute::context;
use coordinate_systems::{Field, Ground};
use framework::MainOutput;
use linear_algebra::{vector, Isometry2, Point2};
use spl_network_messages::PlayerNumber;
use types::{
    ball_position::HypotheticalBallPosition,
    calibration::CalibrationCommand,
    fall_state::FallState,
    filtered_game_controller_state::FilteredGameControllerState,
    kick_decision::KickDecision,
    obstacles::Obstacle,
    primary_state::PrimaryState,
    roles::Role,
    rule_obstacles::RuleObstacle,
    step_plan::Step,
    world_state::{BallState, RobotState, WorldState},
};

#[derive(Deserialize, Serialize)]
pub struct WorldStateComposer {}

#[context]
pub struct CreationContext {}

#[context]
pub struct CycleContext {
    ball: Input<Option<BallState>, "ball_state?">,
    hypothetical_ball_position:
        Input<Vec<HypotheticalBallPosition<Ground>>, "hypothetical_ball_positions">,
    rule_ball: Input<Option<BallState>, "rule_ball_state?">,
    filtered_game_controller_state:
        Input<Option<FilteredGameControllerState>, "filtered_game_controller_state?">,
    ground_to_field: Input<Option<Isometry2<Ground, Field>>, "ground_to_field?">,
    suggested_search_position: Input<Option<Point2<Field>>, "suggested_search_position?">,
    kick_decisions: Input<Option<Vec<KickDecision>>, "kick_decisions?">,
    instant_kick_decisions: Input<Option<Vec<KickDecision>>, "instant_kick_decisions?">,
    walk_return_offset: CyclerState<Step, "walk_return_offset">,

    player_number: Parameter<PlayerNumber, "player_number">,

    fall_state: Input<FallState, "fall_state">,
    has_ground_contact: Input<bool, "has_ground_contact">,
    obstacles: Input<Vec<Obstacle>, "obstacles">,
    rule_obstacles: Input<Vec<RuleObstacle>, "rule_obstacles">,
    primary_state: Input<PrimaryState, "primary_state">,
    role: Input<Role, "role">,
    position_of_interest: Input<Point2<Ground>, "position_of_interest">,
    calibration_command: Input<Option<CalibrationCommand>, "calibration_command?">,
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
            ground_to_field: context.ground_to_field.copied(),
            role: *context.role,
            primary_state: *context.primary_state,
            fall_state: *context.fall_state,
            has_ground_contact: *context.has_ground_contact,
            player_number: *context.player_number,
            walk_return_offset: Isometry2::from_parts(
                vector![
                    context.walk_return_offset.forward,
                    context.walk_return_offset.left
                ],
                context.walk_return_offset.turn,
            ),
        };

        let world_state = WorldState {
            ball: context.ball.copied(),
            rule_ball: context.rule_ball.copied(),
            suggested_search_position: context.suggested_search_position.copied(),
            obstacles: context.obstacles.clone(),
            rule_obstacles: context.rule_obstacles.clone(),
            position_of_interest: *context.position_of_interest,
            robot,
            kick_decisions: context.kick_decisions.cloned(),
            instant_kick_decisions: context.instant_kick_decisions.cloned(),
            filtered_game_controller_state: context.filtered_game_controller_state.copied(),
            hypothetical_ball_positions: context.hypothetical_ball_position.clone(),
            calibration_command: context.calibration_command.copied(),
        };

        Ok(MainOutputs {
            world_state: world_state.into(),
        })
    }
}
