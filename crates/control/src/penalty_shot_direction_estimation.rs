use color_eyre::Result;
use context_attribute::context;
use framework::MainOutput;
use serde::{Deserialize, Serialize};
use spl_network_messages::GamePhase;
use types::{
    ball_position::BallPosition, field_dimensions::FieldDimensions,
    game_controller_state::GameControllerState, penalty_shot_direction::PenaltyShotDirection,
    primary_state::PrimaryState,
};

#[derive(Deserialize, Serialize)]
pub struct PenaltyShotDirectionEstimation {
    last_shot_direction: PenaltyShotDirection,
}

#[context]
pub struct CreationContext {
    pub field_dimensions: Parameter<FieldDimensions, "field_dimensions">,
    pub moving_distance_threshold:
        Parameter<f32, "penalty_shot_direction_estimation.moving_distance_threshold">,
    pub minimum_velocity_threshold:
        Parameter<f32, "penalty_shot_direction_estimation.minimum_velocity_threshold">,
}

#[context]
pub struct CycleContext {
    field_dimensions: Parameter<FieldDimensions, "field_dimensions">,
    moving_distance_threshold:
        Parameter<f32, "penalty_shot_direction_estimation.moving_distance_threshold">,
    pub minimum_velocity_threshold:
        Parameter<f32, "penalty_shot_direction_estimation.minimum_velocity_threshold">,
    pub side_jump_threshold:
        Parameter<f32, "penalty_shot_direction_estimation.side_jump_threshold">,

    ball_position: RequiredInput<Option<BallPosition>, "ball_position?">,
    game_controller_state: RequiredInput<Option<GameControllerState>, "game_controller_state?">,
    primary_state: Input<PrimaryState, "primary_state">,
}

#[context]
#[derive(Default)]
pub struct MainOutputs {
    pub penalty_shot_direction: MainOutput<Option<PenaltyShotDirection>>,
}

impl PenaltyShotDirectionEstimation {
    pub fn new(_context: CreationContext) -> Result<Self> {
        Ok(Self {
            last_shot_direction: PenaltyShotDirection::NotMoving,
        })
    }

    pub fn cycle(&mut self, context: CycleContext) -> Result<MainOutputs> {
        match (
            context.primary_state,
            context.game_controller_state.game_phase,
        ) {
            (PrimaryState::Set, GamePhase::PenaltyShootout { .. }) => {
                self.last_shot_direction = PenaltyShotDirection::NotMoving;
                Ok(MainOutputs::default())
            }
            (PrimaryState::Playing, GamePhase::PenaltyShootout { .. }) => {
                if let PenaltyShotDirection::NotMoving = self.last_shot_direction {
                    let is_ball_position_exceeding_moving_distance_threshold =
                        (context.ball_position.position.x
                            - context.field_dimensions.penalty_marker_distance)
                            .abs()
                            > *context.moving_distance_threshold;
                    let is_ball_velocity_towards_robot = context.ball_position.velocity.x < 0.0;
                    let is_ball_speed_above_the_threshold =
                        context.ball_position.velocity.norm() > *context.minimum_velocity_threshold;
                    let is_ball_position_y_exceeding_center_threshold =
                        context.ball_position.position.y.abs() > *context.side_jump_threshold;

                    if is_ball_position_exceeed_moving_distance_threshold
                        && is_ball_velocity_towards_robot
                        && is_ball_speed_above_the_threshold
                        && is_ball_position_y_exceeding_center_threshold
                    {
                        if context.ball_position.position.y >= 0.0 {
                            self.last_shot_direction = PenaltyShotDirection::Left;
                        } else {
                            self.last_shot_direction = PenaltyShotDirection::Right;
                        }
                    }
                }
                Ok(MainOutputs {
                    penalty_shot_direction: Some(self.last_shot_direction).into(),
                })
            }
            _ => Ok(MainOutputs::default()),
        }
    }
}
