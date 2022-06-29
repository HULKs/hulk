use macros::{module, require_some};

use anyhow::Result;
use nalgebra::{Isometry2, Point2};

use crate::{
    control::filtering::greater_than_with_hysteresis,
    types::{
        BallPosition, BallState, FallState, Obstacle, PrimaryState, RobotState, Role, SensorData,
        Side, WorldState,
    },
};

pub struct WorldStateComposer {
    last_ball_field_side: Side,
}

#[module(control)]
#[input(path = sensor_data, data_type = SensorData)]
#[input(path = ball_position, data_type = BallPosition)]
#[input(path = fall_state, data_type = FallState)]
#[input(path = robot_to_field, data_type = Isometry2<f32>)]
#[input(path = role, data_type = Role)]
#[input(path = primary_state, data_type = PrimaryState)]
#[input(path = has_ground_contact, data_type = bool)]
#[input(path = team_ball, data_type = BallPosition)]
#[input(path = obstacles, data_type = Vec<Obstacle>)]
#[main_output(data_type = WorldState)]
impl WorldStateComposer {}

impl WorldStateComposer {
    fn new(_context: NewContext) -> anyhow::Result<Self> {
        Ok(Self {
            last_ball_field_side: Side::Left,
        })
    }

    fn cycle(&mut self, context: CycleContext) -> Result<MainOutputs> {
        let fall_state = *require_some!(context.fall_state);
        let primary_state = *require_some!(context.primary_state);
        let has_ground_contact = *require_some!(context.has_ground_contact);
        let role = *require_some!(context.role);
        let obstacles = require_some!(context.obstacles);

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
            )),
            (_, None, Some(ball_position), Some(robot_to_field)) => Some(create_ball_state(
                robot_to_field.inverse() * ball_position.position,
                Some(*robot_to_field),
                &mut self.last_ball_field_side,
            )),
            _ => None,
        };

        let robot = RobotState {
            robot_to_field: *context.robot_to_field,
            role,
            primary_state,
            fall_state,
            has_ground_contact,
        };
        let world_state = WorldState {
            ball,
            robot,
            obstacles: obstacles.clone(),
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
    }
}
