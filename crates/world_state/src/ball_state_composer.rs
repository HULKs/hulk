use std::time::SystemTime;

use color_eyre::Result;
use context_attribute::context;
use coordinate_systems::{Field, Ground};
use filtering::hysteresis::greater_than_with_hysteresis;
use framework::{AdditionalOutput, MainOutput};
use linear_algebra::{point, Isometry2, Point2, Vector2};
use serde::{Deserialize, Serialize};
use spl_network_messages::{GamePhase, SubState, Team};
use types::{
    ball_position::BallPosition,
    cycle_time::CycleTime,
    field_dimensions::{FieldDimensions, Side},
    filtered_game_controller_state::FilteredGameControllerState,
    penalty_shot_direction::PenaltyShotDirection,
    primary_state::PrimaryState,
    world_state::{BallState, LastBallState},
};

#[derive(Deserialize, Serialize)]
pub struct BallStateComposer {
    last_ball_field_side: Side,
}

#[context]
pub struct CreationContext {}

#[context]
pub struct CycleContext {
    cycle_time: Input<CycleTime, "cycle_time">,
    ball_position: Input<Option<BallPosition<Ground>>, "ball_position?">,
    penalty_shot_direction: Input<Option<PenaltyShotDirection>, "penalty_shot_direction?">,
    ground_to_field: Input<Option<Isometry2<Ground, Field>>, "ground_to_field?">,
    team_ball: Input<Option<BallPosition<Field>>, "team_ball?">,
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
        // let ball = match (
        //     context.ball_position,
        //     context.team_ball,
        //     context.ground_to_field,
        // ) {
        //     (Some(ball_position), _, Some(ground_to_field)) => Some(create_ball_state(
        //         ball_position.position,
        //         ground_to_field * ball_position.position,
        //         ball_position.velocity,
        //         ball_position.last_seen,
        //         &mut self.last_ball_field_side,
        //         context.penalty_shot_direction.copied(),
        //     )),
        //     (None, Some(team_ball), Some(ground_to_field)) => Some(create_ball_state(
        //         ground_to_field.inverse() * team_ball.position,
        //         team_ball.position,
        //         ground_to_field.inverse() * team_ball.velocity,
        //         team_ball.last_seen,
        //         &mut self.last_ball_field_side,
        //         context.penalty_shot_direction.copied(),
        //     )),
        //     _ => None,
        // };
        let ball = None;

        Ok(MainOutputs {
            ball_state: ball.into(),
        })
    }
}

fn create_ball_state(
    ball_in_ground: Point2<Ground>,
    ball_in_field: Point2<Field>,
    ball_in_ground_velocity: Vector2<Ground>,
    last_seen_ball: SystemTime,
    last_ball_field_side: &mut Side,
    penalty_shot_direction: Option<PenaltyShotDirection>,
) -> BallState {
    let was_in_left_half = *last_ball_field_side == Side::Left;
    let is_in_left_half =
        greater_than_with_hysteresis(was_in_left_half, ball_in_field.y(), 0.0, 0.2);
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
        last_seen_ball,
        field_side,
        penalty_shot_direction,
    }
}
