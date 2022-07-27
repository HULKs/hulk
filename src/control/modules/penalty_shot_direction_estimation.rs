use anyhow::Result;
use module_derive::module;
use spl_network::GamePhase;
use types::{
    BallPosition, FieldDimensions, GameControllerState, PenaltyShotDirection, PrimaryState,
};

pub struct PenaltyShotDirectionEstimation {
    last_shot_direction: PenaltyShotDirection,
}

#[module(control)]
#[input(path = primary_state, data_type = PrimaryState, required)]
#[input(path = game_controller_state, data_type = GameControllerState, required)]
#[input(path = ball_position, data_type = BallPosition, required)]
#[parameter(path = field_dimensions, data_type = FieldDimensions)]
#[parameter(path = control.penalty_shot_direction_estimation.moving_distance_threshold, data_type = f32)]
#[main_output(name = penalty_shot_direction, data_type = PenaltyShotDirection)]
impl PenaltyShotDirectionEstimation {}

impl PenaltyShotDirectionEstimation {
    fn new(_context: NewContext) -> anyhow::Result<Self> {
        Ok(Self {
            last_shot_direction: PenaltyShotDirection::NotMoving,
        })
    }

    fn cycle(&mut self, context: CycleContext) -> Result<MainOutputs> {
        match (
            *context.primary_state,
            context.game_controller_state.game_phase,
        ) {
            (PrimaryState::Set, GamePhase::PenaltyShootout { .. }) => {
                self.last_shot_direction = PenaltyShotDirection::NotMoving;
                Ok(MainOutputs::none())
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
                    penalty_shot_direction: Some(self.last_shot_direction),
                })
            }
            _ => Ok(MainOutputs::none()),
        }
    }
}
