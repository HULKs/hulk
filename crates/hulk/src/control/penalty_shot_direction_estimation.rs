use color_eyre::Result;
use context_attribute::context;
use framework::MainOutput;
use spl_network_messages::GamePhase;
use types::{
    BallPosition, FieldDimensions, GameControllerState, PenaltyShotDirection, PrimaryState,
};

pub struct PenaltyShotDirectionEstimation {
    last_shot_direction: PenaltyShotDirection,
}

#[context]
pub struct CreationContext {
    pub field_dimensions: Parameter<FieldDimensions, "field_dimensions">,
    pub moving_distance_threshold:
        Parameter<f32, "penalty_shot_direction_estimation.moving_distance_threshold">,
}

#[context]
pub struct CycleContext {
    pub field_dimensions: Parameter<FieldDimensions, "field_dimensions">,
    pub moving_distance_threshold:
        Parameter<f32, "penalty_shot_direction_estimation.moving_distance_threshold">,

    pub ball_position: RequiredInput<Option<BallPosition>, "ball_position?">,
    pub game_controller_state: RequiredInput<Option<GameControllerState>, "game_controller_state?">,
    pub primary_state: Input<PrimaryState, "primary_state">,
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
                    if (context.ball_position.position.x
                        - context.field_dimensions.penalty_marker_distance)
                        .abs()
                        > *context.moving_distance_threshold
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
