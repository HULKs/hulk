use std::time::{Duration, SystemTime};

use color_eyre::Result;
use context_attribute::context;
use serde::{Deserialize, Serialize};

use framework::{AdditionalOutput, MainOutput};
use spl_network_messages::{GameState, SubState, Team};
use types::{
    cycle_time::CycleTime,
    filtered_whistle::FilteredWhistle,
    game_controller_state::GameControllerState,
    world_state::{BallState, LastBallState},
};

#[derive(Deserialize, Serialize)]
pub struct KickingTeamFilter {
    last_observed_ball: Option<(SystemTime, BallState)>,
}

#[context]
pub struct CreationContext {}

#[context]
pub struct CycleContext {
    cycle_time: Input<CycleTime, "cycle_time">,
    last_ball_state: CyclerState<LastBallState, "last_ball_state">,
    game_controller_state: RequiredInput<Option<GameControllerState>, "game_controller_state?">,
    filtered_whistle: Input<FilteredWhistle, "filtered_whistle">,

    last_observed_ball: AdditionalOutput<Option<(SystemTime, BallState)>, "last_observed_ball">,

    duration_to_keep_observed_ball:
        Parameter<Duration, "kicking_team_filter.duration_to_keep_observed_ball">,
}

#[context]
pub struct MainOutputs {
    pub filtered_kicking_team: MainOutput<Option<Team>>,
}

impl KickingTeamFilter {
    pub fn new(_context: CreationContext) -> Result<Self> {
        Ok(KickingTeamFilter {
            last_observed_ball: None,
        })
    }

    pub fn cycle(&mut self, mut context: CycleContext) -> Result<MainOutputs> {
        let filtered_kicking_team = self.find_kicking_team(&context);
        context
            .last_observed_ball
            .fill_if_subscribed(|| self.last_observed_ball.clone());

        Ok(MainOutputs {
            filtered_kicking_team: filtered_kicking_team.into(),
        })
    }

    fn find_kicking_team(&mut self, context: &CycleContext) -> Option<Team> {
        let game_controller_state = context.game_controller_state;

        if let Some(kicking_team) = game_controller_state.kicking_team {
            return Some(kicking_team);
        }

        if let LastBallState::LastBall { time, ball } = *context.last_ball_state {
            self.last_observed_ball = Some((time, ball));
        };

        let (time, ball) = self.last_observed_ball?;
        let is_not_in_penalty_kick = game_controller_state.sub_state != Some(SubState::PenaltyKick);

        if is_not_in_penalty_kick
            && context
                .cycle_time
                .start_time
                .duration_since(time)
                .expect("time ran backwards")
                > *context.duration_to_keep_observed_ball
        {
            self.last_observed_ball = None;
            return None;
        }

        let ball_is_in_opponent_half = ball.ball_in_field.x().is_sign_positive();

        match game_controller_state {
            GameControllerState {
                sub_state: Some(SubState::CornerKick),
                ..
            } if ball_is_in_opponent_half => Some(Team::Hulks),
            GameControllerState {
                sub_state: Some(SubState::CornerKick),
                ..
            } if !ball_is_in_opponent_half => Some(Team::Opponent),
            GameControllerState {
                sub_state: Some(SubState::GoalKick),
                ..
            } if ball_is_in_opponent_half => Some(Team::Opponent),
            GameControllerState {
                sub_state: Some(SubState::GoalKick),
                ..
            } if !ball_is_in_opponent_half => Some(Team::Hulks),
            GameControllerState {
                sub_state: Some(SubState::PenaltyKick),
                ..
            } if ball_is_in_opponent_half => Some(Team::Hulks),
            GameControllerState {
                sub_state: Some(SubState::PenaltyKick),
                ..
            } if !ball_is_in_opponent_half => Some(Team::Opponent),
            GameControllerState {
                game_state: GameState::Playing,
                sub_state: None,
                ..
            } => match (
                context.filtered_whistle.is_detected,
                ball_is_in_opponent_half,
            ) {
                (true, false) => Some(Team::Opponent),
                (true, true) => Some(Team::Hulks),
                _ => None,
            },
            _ => None,
        }
    }
}
