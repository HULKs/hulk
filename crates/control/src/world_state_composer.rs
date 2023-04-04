use color_eyre::Result;
use context_attribute::context;
use filtering::hysteresis::greater_than_with_hysteresis;
use framework::MainOutput;
use nalgebra::{Isometry2, Point2};
use spl_network_messages::PlayerNumber;
use types::{
    BallPosition, BallState, FallState, FilteredGameState, GameControllerState, Obstacle,
    PenaltyShotDirection, PrimaryState, RobotState, Role, Side, WorldState,
};

pub struct WorldStateComposer {
    last_ball_field_side: Side,
}

#[context]
pub struct CreationContext {
    pub player_number: Parameter<PlayerNumber, "player_number">,
}

#[context]
pub struct CycleContext {
    pub ball_position: Input<Option<BallPosition>, "ball_position?">,
    pub filtered_game_state: Input<Option<FilteredGameState>, "filtered_game_state?">,
    pub game_controller_state: Input<Option<GameControllerState>, "game_controller_state?">,
    pub penalty_shot_direction: Input<Option<PenaltyShotDirection>, "penalty_shot_direction?">,
    pub robot_to_field: Input<Option<Isometry2<f32>>, "robot_to_field?">,
    pub team_ball: Input<Option<BallPosition>, "team_ball?">,

    pub player_number: Parameter<PlayerNumber, "player_number">,

    pub fall_state: Input<FallState, "fall_state">,
    pub has_ground_contact: Input<bool, "has_ground_contact">,
    pub obstacles: Input<Vec<Obstacle>, "obstacles">,
    pub primary_state: Input<PrimaryState, "primary_state">,
    pub role: Input<Role, "role">,
}

#[context]
#[derive(Default)]
pub struct MainOutputs {
    pub world_state: MainOutput<WorldState>,
}

impl WorldStateComposer {
    pub fn new(_context: CreationContext) -> Result<Self> {
        Ok(Self {
            last_ball_field_side: Side::Left,
        })
    }

    pub fn cycle(&mut self, context: CycleContext) -> Result<MainOutputs> {
        let ball = match (
            context.primary_state,
            context.ball_position,
            context.team_ball,
            context.robot_to_field,
        ) {
            (PrimaryState::Ready, ..) => None,
            (_, Some(ball_position), _, robot_to_field) => Some(create_ball_state(
                ball_position.position,
                robot_to_field,
                &mut self.last_ball_field_side,
                context.penalty_shot_direction.copied(),
            )),
            (_, None, Some(ball_position), Some(robot_to_field)) => Some(create_ball_state(
                robot_to_field.inverse() * ball_position.position,
                Some(robot_to_field),
                &mut self.last_ball_field_side,
                context.penalty_shot_direction.copied(),
            )),
            _ => None,
        };

        let robot = RobotState {
            robot_to_field: context.robot_to_field.copied(),
            role: *context.role,
            primary_state: *context.primary_state,
            fall_state: *context.fall_state,
            has_ground_contact: *context.has_ground_contact,
            player_number: *context.player_number,
        };

        let world_state = WorldState {
            ball,
            filtered_game_state: context.filtered_game_state.copied(),
            obstacles: context.obstacles.clone(),
            robot,
            game_controller_state: context.game_controller_state.copied(),
        };

        Ok(MainOutputs {
            world_state: world_state.into(),
        })
    }
}

fn create_ball_state(
    position: Point2<f32>,
    robot_to_field: Option<&Isometry2<f32>>,
    last_ball_field_side: &mut Side,
    penalty_shot_direction: Option<PenaltyShotDirection>,
) -> BallState {
    let was_in_left_half = *last_ball_field_side == Side::Left;
    let field_side = match robot_to_field {
        Some(robot_to_field) => {
            let ball_in_field = robot_to_field * position;
            let is_in_left_half =
                greater_than_with_hysteresis(was_in_left_half, ball_in_field.y, 0.0, 0.1);
            let field_side = if is_in_left_half {
                Side::Left
            } else {
                Side::Right
            };
            *last_ball_field_side = field_side;
            field_side
        }
        None => Side::Left,
    };
    BallState {
        position,
        field_side,
        penalty_shot_direction,
    }
}
