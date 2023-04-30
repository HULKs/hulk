use color_eyre::Result;
use context_attribute::context;
use filtering::hysteresis::greater_than_with_hysteresis;
use framework::MainOutput;
use nalgebra::{point, Isometry2, Point2, Vector2};
use spl_network_messages::{SubState, Team};
use types::{
    BallPosition, BallState, FieldDimensions, GameControllerState, PenaltyShotDirection,
    PrimaryState, Side,
};

pub struct BallStateComposer {
    last_ball_field_side: Side,
}

#[context]
pub struct CreationContext {}

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
    pub rule_ball_state: MainOutput<Option<BallState>>,
}

impl BallStateComposer {
    pub fn new(_context: CreationContext) -> Result<Self> {
        Ok(Self {
            last_ball_field_side: Side::Left,
        })
    }

    pub fn cycle(&mut self, context: CycleContext) -> Result<MainOutputs> {
        let ball = match (
            context.ball_position,
            context.team_ball,
            context.robot_to_field,
        ) {
            (Some(ball_position), _, Some(robot_to_field)) => Some(create_ball_state(
                ball_position.position,
                robot_to_field * ball_position.position,
                ball_position.velocity,
                &mut self.last_ball_field_side,
                context.penalty_shot_direction.copied(),
            )),
            (None, Some(ball_position), Some(robot_to_field)) => Some(create_ball_state(
                robot_to_field.inverse() * ball_position.position,
                ball_position.position,
                ball_position.velocity,
                &mut self.last_ball_field_side,
                context.penalty_shot_direction.copied(),
            )),
            _ => None,
        };

        let rule_ball = match (
            context.primary_state,
            context.robot_to_field,
            context.game_controller_state,
        ) {
            (
                PrimaryState::Ready,
                Some(robot_to_field),
                Some(GameControllerState {
                    sub_state: Some(SubState::PenaltyKick),
                    kicking_team,
                    ..
                }),
            ) => {
                let side_factor = match kicking_team {
                    Team::Opponent => -1.0,
                    Team::Hulks => 1.0,
                    // If uncertain get ready to defend own goal
                    Team::Uncertain => -1.0,
                };
                let penalty_spot_x = context.field_dimensions.length / 2.0
                    - context.field_dimensions.penalty_marker_distance;
                let penalty_spot_location = point![side_factor * penalty_spot_x, 0.0];
                Some(create_ball_state(
                    robot_to_field.inverse() * penalty_spot_location,
                    penalty_spot_location,
                    Vector2::zeros(),
                    &mut self.last_ball_field_side,
                    context.penalty_shot_direction.copied(),
                ))
            }
            (PrimaryState::Ready, Some(robot_to_field), ..) => Some(create_ball_state(
                robot_to_field.inverse() * Point2::origin(),
                Point2::origin(),
                Vector2::zeros(),
                &mut self.last_ball_field_side,
                context.penalty_shot_direction.copied(),
            )),
            _ => None,
        };

        Ok(MainOutputs {
            ball_state: ball.into(),
            rule_ball_state: rule_ball.into(),
        })
    }
}

fn create_ball_state(
    ball_in_ground: Point2<f32>,
    ball_in_field: Point2<f32>,
    ball_in_ground_velocity: Vector2<f32>,
    last_ball_field_side: &mut Side,
    penalty_shot_direction: Option<PenaltyShotDirection>,
) -> BallState {
    let was_in_left_half = *last_ball_field_side == Side::Left;
    let is_in_left_half = greater_than_with_hysteresis(was_in_left_half, ball_in_field.y, 0.0, 0.1);
    let side = if is_in_left_half {
        Side::Left
    } else {
        Side::Right
    };
    *last_ball_field_side = side;
    let field_side = side;
    BallState {
        ball_in_ground,
        ball_in_field,
        ball_in_ground_velocity,
        field_side,
        penalty_shot_direction,
    }
}
