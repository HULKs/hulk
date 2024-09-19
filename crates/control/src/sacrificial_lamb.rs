use color_eyre::Result;
use std::{
    collections::BTreeMap,
    time::{Duration, SystemTime},
};

use context_attribute::context;
use framework::{AdditionalOutput, MainOutput, PerceptionInput};
use serde::{Deserialize, Serialize};
use spl_network_messages::{GameControllerStateMessage, Penalty, PlayerNumber};
use types::{cycle_time::CycleTime, messages::IncomingMessage, pose_detection::VisualRefereeState};

#[derive(Deserialize, Serialize)]
pub struct SacrificialLamb {
    last_majority_vote_verdict: bool,
    visual_referee_state: VisualRefereeState,
    motion_in_standby_count: usize,
}

#[context]
pub struct CreationContext {}

#[context]
pub struct CycleContext {
    network_message: PerceptionInput<Option<IncomingMessage>, "SplNetwork", "filtered_message?">,

    cycle_time: Input<CycleTime, "cycle_time">,
    majority_vote_is_referee_ready_pose_detected:
        Input<bool, "majority_vote_is_referee_ready_pose_detected">,

    walk_in_position_index: Input<usize, "walk_in_position_index">,
    wait_for_opponent_penalties_period:
        Parameter<Duration, "sacrificial_lamb.wait_for_opponent_penalties_period">,
    wait_for_own_penalties_period:
        Parameter<Duration, "sacrificial_lamb.wait_for_own_penalties_period">,
    sacrificial_lamb: Parameter<usize, "sacrificial_lamb.sacrificial_nao_position_number">,

    visual_referee_state: AdditionalOutput<VisualRefereeState, "visual_referee_state">,
}

#[context]
#[derive(Default)]
pub struct MainOutputs {
    pub visual_referee_proceed_to_ready: MainOutput<bool>,
}

impl SacrificialLamb {
    pub fn new(_context: CreationContext) -> Result<Self> {
        Ok(Self {
            last_majority_vote_verdict: false,
            visual_referee_state: VisualRefereeState::WaitingForDetections,
            motion_in_standby_count: 0,
        })
    }

    pub fn cycle(&mut self, mut context: CycleContext) -> Result<MainOutputs> {
        let cycle_start_time = context.cycle_time.start_time;

        let game_controller_message =
            unpack_game_controller_messages(&context.network_message.persistent);

        let new_motion_in_standby_count = game_controller_message
            .iter()
            .map(|message| {
                message
                    .hulks_team
                    .players
                    .iter()
                    .filter(|player| {
                        matches!(player.penalty, Some(Penalty::IllegalMotionInStandby { .. }))
                    })
                    .count()
                    + message
                        .opponent_team
                        .players
                        .iter()
                        .filter(|player| {
                            matches!(player.penalty, Some(Penalty::IllegalMotionInStandby { .. }))
                        })
                        .count()
            })
            .max();

        let current_majority_vote_verdict = !self.last_majority_vote_verdict
            && *context.majority_vote_is_referee_ready_pose_detected;
        self.last_majority_vote_verdict = *context.majority_vote_is_referee_ready_pose_detected;

        let motion_in_standby =
            new_motion_in_standby_count.map_or(false, |new_motion_in_standby_count| {
                let motion_in_standby = new_motion_in_standby_count > self.motion_in_standby_count;
                self.motion_in_standby_count = new_motion_in_standby_count;
                motion_in_standby
            });

        self.visual_referee_state = match (
            self.visual_referee_state,
            current_majority_vote_verdict,
            motion_in_standby,
        ) {
            (VisualRefereeState::WaitingForDetections, true, motion_in_standby) => {
                if motion_in_standby {
                    VisualRefereeState::WaitingForDetections
                } else {
                    VisualRefereeState::WaitingForOpponentPenalties {
                        active_since: cycle_start_time,
                    }
                }
            }
            (VisualRefereeState::WaitingForOpponentPenalties { .. }, _, true) => {
                VisualRefereeState::WaitingForDetections
            }
            (VisualRefereeState::WaitingForOpponentPenalties { active_since }, _, false) => {
                if cycle_start_time
                    .duration_since(active_since)
                    .expect("time ran backwards")
                    >= *context.wait_for_opponent_penalties_period
                {
                    if context.walk_in_position_index == context.sacrificial_lamb {
                        VisualRefereeState::GoToReady
                    } else {
                        VisualRefereeState::WaitingForOwnPenalties {
                            active_since: cycle_start_time,
                        }
                    }
                } else {
                    VisualRefereeState::WaitingForOpponentPenalties { active_since }
                }
            }
            (VisualRefereeState::WaitingForOwnPenalties { .. }, _, true) => {
                VisualRefereeState::WaitingForDetections
            }
            (VisualRefereeState::WaitingForOwnPenalties { active_since }, _, false) => {
                if cycle_start_time
                    .duration_since(active_since)
                    .expect("time ran backwards")
                    >= *context.wait_for_own_penalties_period
                {
                    VisualRefereeState::GoToReady
                } else {
                    VisualRefereeState::WaitingForOwnPenalties { active_since }
                }
            }
            (current_state, _, _) => current_state,
        };

        context
            .visual_referee_state
            .fill_if_subscribed(|| self.visual_referee_state);

        Ok(MainOutputs {
            visual_referee_proceed_to_ready: (self.visual_referee_state
                == VisualRefereeState::GoToReady)
                .into(),
        })
    }
}

fn unpack_game_controller_messages<'a>(
    message_tree: &BTreeMap<SystemTime, Vec<Option<&'a IncomingMessage>>>,
) -> Vec<&'a GameControllerStateMessage> {
    message_tree
        .values()
        .flatten()
        .filter_map(|message| match message {
            Some(IncomingMessage::GameController(_, message)) => Some(message),
            Some(IncomingMessage::Spl(..)) | None => None,
        })
        .collect()
}
