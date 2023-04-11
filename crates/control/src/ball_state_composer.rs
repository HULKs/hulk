use color_eyre::Result;
use context_attribute::context;
use filtering::hysteresis::greater_than_with_hysteresis;
use framework::MainOutput;
use nalgebra::{point, Isometry2, Point2};
use spl_network_messages::{SubState, Team};
use types::{
    BallPosition, BallState, FieldDimensions, GameControllerState, PenaltyShotDirection,
    PrimaryState, Side,
};

pub struct BallStateComposer {
    last_ball_field_side: Side,
}

#[context]
pub struct CreationContext {
    pub field_dimensions: Parameter<FieldDimensions, "field_dimensions">,
}

#[context]
pub struct CycleContext {
    pub ball_position: Input<Option<BallPosition>, "ball_position?">,
    pub penalty_shot_direction: Input<Option<PenaltyShotDirection>, "penalty_shot_direction?">,
    pub robot_to_field: Input<Option<Isometry2<f32>>, "robot_to_field?">,
    pub team_ball: Input<Option<BallPosition>, "team_ball?">,
    pub primary_state: Input<PrimaryState, "primary_state">,
    pub game_controller_state: Input<Option<GameControllerState>, "game_controller_state?">,
    pub field_dimensions: Parameter<FieldDimensions, "field_dimensions">,
}

#[context]
#[derive(Default)]
pub struct MainOutputs {
    pub ball_state: MainOutput<Option<BallState>>,
}

impl BallStateComposer {
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
            (PrimaryState::Ready, _, _, Some(robot_to_field)) => {
                if let Some(game_controller_state) = context.game_controller_state {
                    match game_controller_state {
                        GameControllerState {
                            sub_state: Some(SubState::PenaltyKick),
                            kicking_team: Team::Opponent,
                            ..
                        } => Some(create_ball_state(
                            robot_to_field.inverse()
                                * point![
                                    -context.field_dimensions.length / 2.0
                                        + context.field_dimensions.penalty_marker_distance,
                                    0.0
                                ],
                            context.robot_to_field,
                            &mut self.last_ball_field_side,
                            context.penalty_shot_direction.copied(),
                        )),
                        GameControllerState {
                            sub_state: Some(SubState::PenaltyKick),
                            kicking_team: Team::Hulks,
                            ..
                        } => Some(create_ball_state(
                            robot_to_field.inverse()
                                * point![
                                    context.field_dimensions.length / 2.0
                                        + context.field_dimensions.penalty_marker_distance,
                                    0.0
                                ],
                            context.robot_to_field,
                            &mut self.last_ball_field_side,
                            context.penalty_shot_direction.copied(),
                        )),
                        _ => None,
                    }
                } else {
                    None
                }
            }
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

        Ok(MainOutputs {
            ball_state: ball.into(),
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
