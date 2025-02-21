use color_eyre::Result;
use context_attribute::context;
use coordinate_systems::{Field, Ground};
use framework::MainOutput;
use linear_algebra::{Isometry2, Point2};
use serde::{Deserialize, Serialize};
use spl_network_messages::{GamePhase, SubState, Team};
use types::{
    ball_position::BallPosition,
    field_dimensions::{FieldDimensions, Half},
    filtered_game_controller_state::FilteredGameControllerState,
    parameters::PenaltyShotDirectionParameters,
    penalty_shot_direction::PenaltyShotDirection,
    primary_state::PrimaryState,
};

#[derive(Deserialize, Serialize)]
pub struct PenaltyShotDirectionEstimation {
    last_shot_direction: PenaltyShotDirection,
    placed_ball_position: Option<Point2<Ground>>,
}

#[context]
pub struct CreationContext {}

#[context]
pub struct CycleContext {
    field_dimensions: Parameter<FieldDimensions, "field_dimensions">,
    penalty_shot_parameters:
        Parameter<PenaltyShotDirectionParameters, "penalty_shot_direction_estimation">,
    minimum_robot_radius_at_foot_height:
        Parameter<f32, "behavior.path_planning.minimum_robot_radius_at_foot_height">,

    ball_position: RequiredInput<Option<BallPosition<Ground>>, "ball_position?">,
    filtered_game_controller_state:
        RequiredInput<Option<FilteredGameControllerState>, "filtered_game_controller_state?">,
    primary_state: Input<PrimaryState, "primary_state">,
    ground_to_field: RequiredInput<Option<Isometry2<Ground, Field>>, "ground_to_field?">,
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
            placed_ball_position: None,
        })
    }

    pub fn cycle(&mut self, context: CycleContext) -> Result<MainOutputs> {
        match (
            context.primary_state,
            context.filtered_game_controller_state.game_phase,
            context.filtered_game_controller_state.sub_state,
            context.filtered_game_controller_state.kicking_team,
        ) {
            (PrimaryState::Set, GamePhase::PenaltyShootout { .. }, ..)
            | (PrimaryState::Set, _, Some(SubState::PenaltyKick), Some(Team::Opponent)) => {
                self.last_shot_direction = PenaltyShotDirection::NotMoving;
                self.placed_ball_position = Some(context.ball_position.position);
                Ok(MainOutputs::default())
            }
            (PrimaryState::Playing, GamePhase::PenaltyShootout { .. }, ..)
            | (PrimaryState::Playing, _, Some(SubState::PenaltyKick), Some(Team::Opponent)) => {
                let penalty_marker_position_in_ground = context.ground_to_field.inverse()
                    * FieldDimensions::penalty_spot(context.field_dimensions, Half::Own);
                let reference_position = self
                    .placed_ball_position
                    .unwrap_or(penalty_marker_position_in_ground);
                let side_jump_threshold =
                    (context.penalty_shot_parameters.moving_distance_threshold
                        * (context.minimum_robot_radius_at_foot_height
                            + context.penalty_shot_parameters.center_jump_trigger_radius))
                        / context.field_dimensions.penalty_marker_distance;
                if let PenaltyShotDirection::NotMoving = self.last_shot_direction {
                    if context.ball_position.velocity.x()
                        <= context.penalty_shot_parameters.minimum_velocity
                    {
                        if context.ball_position.position.y() - reference_position.y()
                            > side_jump_threshold
                        {
                            self.last_shot_direction = PenaltyShotDirection::Left
                        } else if context.ball_position.position.y() - reference_position.y()
                            < -side_jump_threshold
                        {
                            self.last_shot_direction = PenaltyShotDirection::Right
                        } else {
                            self.last_shot_direction = PenaltyShotDirection::Center
                        }
                    }
                }
                Ok(MainOutputs {
                    penalty_shot_direction: Some(self.last_shot_direction).into(),
                })
            }
            _ => {
                self.placed_ball_position = None;
                Ok(MainOutputs::default())
            }
        }
    }
}
