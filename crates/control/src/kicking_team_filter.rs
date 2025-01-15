use color_eyre::Result;
use serde::{Deserialize, Serialize};

use context_attribute::context;
use framework::MainOutput;
use spl_network_messages::{SubState, Team};
use types::{game_controller_state::GameControllerState, world_state::BallState};

#[derive(Deserialize, Serialize)]
pub struct KickingTeamFilter {}

#[context]
pub struct CreationContext {}

#[context]
pub struct CycleContext {
    last_ball_state: CyclerState<BallState, "last_ball_position">,
    game_controller_state: RequiredInput<Option<GameControllerState>, "game_controller_state?">,
    detected_free_kick_kicking_team: Input<Option<Team>, "detected_free_kick_kicking_team?">,
}

#[context]
pub struct MainOutputs {
    pub filtered_kicking_team: MainOutput<Option<Team>>,
}

impl KickingTeamFilter {
    pub fn new(_context: CreationContext) -> Result<Self> {
        Ok(KickingTeamFilter {})
    }

    pub fn cycle(&mut self, context: CycleContext) -> Result<MainOutputs> {
        let game_controller_state = context.game_controller_state;
        let sub_state = context.game_controller_state.sub_state;
        let last_ball_state = context.last_ball_state;

        let filtered_kicking_team = if game_controller_state.kicking_team.is_some() {
            game_controller_state.kicking_team
        } else if context.detected_free_kick_kicking_team.is_some() {
            context.detected_free_kick_kicking_team.copied()
        } else {
            match sub_state {
                Some(SubState::CornerKick)
                    if last_ball_state.ball_in_field.x().is_sign_positive() =>
                {
                    Some(Team::Hulks)
                }
                Some(SubState::CornerKick)
                    if last_ball_state.ball_in_field.x().is_sign_negative() =>
                {
                    Some(Team::Opponent)
                }
                Some(SubState::GoalKick)
                    if last_ball_state.ball_in_field.x().is_sign_positive() =>
                {
                    Some(Team::Opponent)
                }
                Some(SubState::GoalKick)
                    if last_ball_state.ball_in_field.x().is_sign_negative() =>
                {
                    Some(Team::Hulks)
                }
                _ => None,
            }
        };
        Ok(MainOutputs {
            filtered_kicking_team: filtered_kicking_team.into(),
        })
    }
}
