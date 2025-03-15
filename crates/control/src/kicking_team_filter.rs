use std::time::{Duration, SystemTime};

use color_eyre::Result;
use serde::{Deserialize, Serialize};

use context_attribute::context;
use framework::{AdditionalOutput, MainOutput};
use spl_network_messages::{GameState, Penalty, SubState, Team};
use types::{
    cycle_time::CycleTime, filtered_whistle::FilteredWhistle,
    game_controller_state::GameControllerState, players::Players, world_state::BallState,
};

#[derive(Deserialize, Serialize)]
pub struct KickingTeamFilter {
    time_last_ball_state_became_default: SystemTime,
    last_ball_state: BallState,
    last_non_default_ball_state: Option<BallState>,
    last_penalties: Players<Option<Penalty>>,
}

#[context]
pub struct CreationContext {}

#[context]
pub struct CycleContext {
    last_ball_state: CyclerState<BallState, "last_ball_state">,
    game_controller_state: RequiredInput<Option<GameControllerState>, "game_controller_state?">,

    cycle_time: Input<CycleTime, "cycle_time">,
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
            time_last_ball_state_became_default: SystemTime::UNIX_EPOCH,
            last_ball_state: Default::default(),
            last_non_default_ball_state: Default::default(),
            last_penalties: Default::default(),
        })
    }

    pub fn cycle(&mut self, mut context: CycleContext) -> Result<MainOutputs> {
        context
            .additonal_last_ball_state
            .fill_if_subscribed(|| *context.last_ball_state);

        let game_controller_state = context.game_controller_state;

        self.update_last_non_default_ball_state(&context);

        let has_hulk_been_penalized = self.has_hulk_been_penalized(&context);

        let filtered_kicking_team = if game_controller_state.kicking_team.is_some() {
            game_controller_state.kicking_team
        } else if let Some(last_non_default_ball_state) = self.last_non_default_ball_state {
            let ball_is_in_opponent_half = !last_non_default_ball_state
                .ball_in_field
                .x()
                .is_sign_negative();
            match game_controller_state {
                GameControllerState {
                    sub_state: Some(SubState::CornerKick),
                    ..
                } => {
                    if ball_is_in_opponent_half {
                        Some(Team::Hulks)
                    } else {
                        Some(Team::Opponent)
                    }
                }
                GameControllerState {
                    sub_state: Some(SubState::GoalKick),
                    ..
                } => {
                    if ball_is_in_opponent_half {
                        Some(Team::Opponent)
                    } else {
                        Some(Team::Hulks)
                    }
                }
                GameControllerState {
                    sub_state: Some(SubState::PenaltyKick),
                    ..
                } => {
                    if ball_is_in_opponent_half {
                        Some(Team::Hulks)
                    } else {
                        Some(Team::Opponent)
                    }
                }
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
                GameControllerState {
                    sub_state: Some(SubState::PushingFreeKick),
                    ..
                } => {
                    if has_hulk_been_penalized {
                        Some(Team::Opponent)
                    } else {
                        Some(Team::Hulks)
                    }
                }
                _ => None,
            }
        } else {
            None
        };
        Ok(MainOutputs {
            filtered_kicking_team: filtered_kicking_team.into(),
        })
    }

    fn update_last_non_default_ball_state(&mut self, context: &CycleContext) {
        let duration_since_last_non_default_ball_state = context
            .cycle_time
            .start_time
            .duration_since(self.time_last_ball_state_became_default)
            .expect("Time ran backwards");

        if *context.last_ball_state == BallState::default()
            && self.last_ball_state != BallState::default()
        {
            self.time_last_ball_state_became_default = context.cycle_time.start_time;
        }

        self.last_ball_state = *context.last_ball_state;

        if *context.last_ball_state != BallState::default() {
            self.last_non_default_ball_state = Some(*context.last_ball_state);
        }

        if duration_since_last_non_default_ball_state
            >= *context.duration_to_keep_non_default_last_ball_state
        {
            self.last_non_default_ball_state = None;
        }
    }

    fn has_hulk_been_penalized(&mut self, context: &CycleContext) -> bool {
        let current_penalites = context.game_controller_state.penalties;

        let has_hulk_been_penalized = current_penalites
            .iter()
            .zip(self.last_penalties.iter())
            .any(|((_, current_penalty), (_, last_penalty))| {
                current_penalty.is_none() && last_penalty.is_some()
            });

        self.last_penalties = current_penalites;

        has_hulk_been_penalized
    }
}
