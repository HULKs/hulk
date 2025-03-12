use std::time::{Duration, SystemTime};

use color_eyre::Result;
use serde::{Deserialize, Serialize};

use context_attribute::context;
use framework::{AdditionalOutput, MainOutput};
use spl_network_messages::{SubState, Team};
use types::{
    filtered_whistle::FilteredWhistle, game_controller_state::GameControllerState,
    world_state::BallState,
};

#[derive(Deserialize, Serialize)]
pub struct KickingTeamFilter {
    time_last_ball_state_became_default: SystemTime,
    last_ball_state: BallState,
    last_non_default_ball_state: Option<BallState>,
}

#[context]
pub struct CreationContext {}

#[context]
pub struct CycleContext {
    last_ball_state: CyclerState<BallState, "last_ball_state">,
    game_controller_state: RequiredInput<Option<GameControllerState>, "game_controller_state?">,
    detected_free_kick_kicking_team: Input<Option<Team>, "detected_free_kick_kicking_team?">,
    filtered_whistle: Input<FilteredWhistle, "filtered_whistle">,

    duration_to_keep_non_default_last_ball_state:
        Parameter<Duration, "kicking_team_filter.duration_to_keep_non_default_last_ball_state">,

    additonal_last_ball_state: AdditionalOutput<BallState, "last_ball_state">,
}

#[context]
pub struct MainOutputs {
    pub filtered_kicking_team: MainOutput<Option<Team>>,
}

impl KickingTeamFilter {
    pub fn new(_context: CreationContext) -> Result<Self> {
        Ok(KickingTeamFilter {
            time_last_ball_state_became_default: SystemTime::now(),
            last_ball_state: Default::default(),
            last_non_default_ball_state: Default::default(),
        })
    }

    pub fn cycle(&mut self, mut context: CycleContext) -> Result<MainOutputs> {
        context
            .additonal_last_ball_state
            .fill_if_subscribed(|| *context.last_ball_state);

        let game_controller_state = context.game_controller_state;
        let sub_state = context.game_controller_state.sub_state;

        let duration_since_last_non_default_ball_state =
            SystemTime::now().duration_since(self.time_last_ball_state_became_default)?;

        if *context.last_ball_state == BallState::default()
            && self.last_ball_state != BallState::default()
        {
            self.time_last_ball_state_became_default = SystemTime::now();
        }
        self.last_ball_state = *context.last_ball_state;

        if *context.last_ball_state != BallState::default()
            || *context.last_ball_state == BallState::default()
                && duration_since_last_non_default_ball_state
                    >= *context.duration_to_keep_non_default_last_ball_state
        {
            self.last_non_default_ball_state = Some(*context.last_ball_state);
        }

        let filtered_kicking_team = if game_controller_state.kicking_team.is_some() {
            game_controller_state.kicking_team
        } else if context.detected_free_kick_kicking_team.is_some() {
            context.detected_free_kick_kicking_team.copied()
        } else if let Some(last_non_default_ball_state) = self.last_non_default_ball_state {
            match sub_state {
                Some(SubState::CornerKick)
                    if last_non_default_ball_state
                        .ball_in_field
                        .x()
                        .is_sign_positive() =>
                {
                    Some(Team::Hulks)
                }
                Some(SubState::CornerKick)
                    if last_non_default_ball_state
                        .ball_in_field
                        .x()
                        .is_sign_negative() =>
                {
                    Some(Team::Opponent)
                }
                Some(SubState::GoalKick)
                    if last_non_default_ball_state
                        .ball_in_field
                        .x()
                        .is_sign_positive() =>
                {
                    Some(Team::Opponent)
                }
                Some(SubState::GoalKick)
                    if last_non_default_ball_state
                        .ball_in_field
                        .x()
                        .is_sign_negative() =>
                {
                    Some(Team::Hulks)
                }
                None => match (
                    context.filtered_whistle.is_detected,
                    last_non_default_ball_state
                        .ball_in_field
                        .x()
                        .is_sign_positive(),
                ) {
                    (true, true) => Some(Team::Opponent),
                    (true, false) => Some(Team::Hulks),
                    _ => None,
                },
                _ => None,
            }
        } else {
            None
        };
        Ok(MainOutputs {
            filtered_kicking_team: filtered_kicking_team.into(),
        })
    }
}
