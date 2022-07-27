use module_derive::module;

use anyhow::Result;
use nalgebra::{Isometry2, Point2};

use spl_network::{GamePhase, PlayerNumber};
use types::{
    BallPosition, BallState, FallState, FilteredGameState, GameControllerState, Obstacle,
    PenaltyShotDirection, PrimaryState, RobotState, Role, Side, WorldState,
};

use crate::control::filtering::greater_than_with_hysteresis;

pub struct WorldStateComposer {
    last_ball_field_side: Side,
}

#[module(control)]
#[input(path = fall_state, data_type = FallState, required)]
#[input(path = has_ground_contact, data_type = bool, required)]
#[input(path = obstacles, data_type = Vec<Obstacle>, required)]
#[input(path = primary_state, data_type = PrimaryState, required)]
#[input(path = ball_position, data_type = BallPosition)]
#[input(path = filtered_game_state, data_type = FilteredGameState)]
#[input(path = penalty_shot_direction, data_type = PenaltyShotDirection)]
#[input(path = game_controller_state, data_type = GameControllerState)]
#[input(path = robot_to_field, data_type = Isometry2<f32>)]
#[input(path = role, data_type = Role, required)]
#[input(path = team_ball, data_type = BallPosition)]
#[parameter(path = player_number, data_type = PlayerNumber)]
#[main_output(data_type = WorldState)]
impl WorldStateComposer {}

impl WorldStateComposer {
    fn new(_context: NewContext) -> anyhow::Result<Self> {
        Ok(Self {
            last_ball_field_side: Side::Left,
        })
    }

    fn cycle(&mut self, context: CycleContext) -> Result<MainOutputs> {
        let fall_state = *context.fall_state;
        let primary_state = *context.primary_state;
        let has_ground_contact = *context.has_ground_contact;
        let role = *context.role;
        let obstacles = context.obstacles;
        let game_controller_state = *context.game_controller_state;
        let game_phase = match context.game_controller_state {
            Some(game_controller_state) => game_controller_state.game_phase,
            None => GamePhase::Normal,
        };

        let ball = match (
            primary_state,
            context.ball_position,
            context.team_ball,
            context.robot_to_field,
        ) {
            (PrimaryState::Ready, ..) => None,
            (_, Some(ball_position), _, robot_to_field) => Some(create_ball_state(
                ball_position.position,
                *robot_to_field,
                &mut self.last_ball_field_side,
                *context.penalty_shot_direction,
            )),
            (_, None, Some(ball_position), Some(robot_to_field)) => Some(create_ball_state(
                robot_to_field.inverse() * ball_position.position,
                Some(*robot_to_field),
                &mut self.last_ball_field_side,
                *context.penalty_shot_direction,
            )),
            _ => None,
        };

        let robot = RobotState {
            robot_to_field: *context.robot_to_field,
            role,
            primary_state,
            fall_state,
            has_ground_contact,
            player_number: *context.player_number,
        };

        let world_state = WorldState {
            ball,
            filtered_game_state: *context.filtered_game_state,
            game_phase,
            obstacles: obstacles.clone(),
            robot,
            game_controller_state,
        };

        Ok(MainOutputs {
            world_state: Some(world_state),
        })
    }
}

fn create_ball_state(
    position: Point2<f32>,
    robot_to_field: Option<Isometry2<f32>>,
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
