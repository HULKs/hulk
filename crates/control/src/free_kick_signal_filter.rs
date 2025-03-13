use std::{
    collections::{BTreeMap, VecDeque},
    sync::Arc,
    time::{Duration, SystemTime},
};

use color_eyre::Result;
use serde::{Deserialize, Serialize};

use context_attribute::context;
use framework::{AdditionalOutput, MainOutput, PerceptionInput};
use hardware::NetworkInterface;
use spl_network_messages::{HulkMessage, PlayerNumber, SubState, Team, VisualRefereeMessage};
use types::{
    cycle_time::CycleTime,
    field_dimensions::GlobalFieldSide,
    game_controller_state::GameControllerState,
    messages::{IncomingMessage, OutgoingMessage},
    players::Players,
    pose_detection::{FreeKickSignalDetectionResult, TimeTaggedKickingTeamDetections},
    pose_kinds::PoseKind,
};

#[derive(Deserialize, Serialize)]
pub struct FreeKickSignalFilter {
    free_kick_signal_detection_times: Players<Option<TimeTaggedKickingTeamDetections>>,
    detected_free_kick_detections_queue: VecDeque<Team>,
    has_sent_free_kick_signal_message: bool,
}

#[context]
pub struct CreationContext {
    referee_pose_queue_length: Parameter<usize, "pose_detection.referee_pose_queue_length">,
}

#[context]
pub struct CycleContext {
    hardware_interface: HardwareInterface,

    referee_pose_kind:
        PerceptionInput<Option<PoseKind>, "ObjectDetectionTop", "referee_pose_kind?">,
    network_message: PerceptionInput<Option<IncomingMessage>, "SplNetwork", "filtered_message?">,
    game_controller_state: RequiredInput<Option<GameControllerState>, "game_controller_state?">,

    cycle_time: Input<CycleTime, "cycle_time">,

    initial_message_grace_period:
        Parameter<Duration, "free_kick_signal_detection_filter.initial_message_grace_period">,
    minimum_free_kick_signal_detections:
        Parameter<usize, "free_kick_signal_detection_filter.minimum_free_kick_signal_detections">,
    player_number: Parameter<PlayerNumber, "player_number">,
    referee_pose_queue_length: Parameter<usize, "pose_detection.referee_pose_queue_length">,
    minimum_number_poses_before_message:
        Parameter<usize, "pose_detection.minimum_number_poses_before_message">,

    free_kick_detection_times: AdditionalOutput<
        Players<Option<TimeTaggedKickingTeamDetections>>,
        "free_kick_detection_times",
    >,
    free_kick_detections_queue: AdditionalOutput<VecDeque<Team>, "free_kick_detections_queue">,
}

#[context]
#[derive(Default)]
pub struct MainOutputs {
    pub did_detect_any_free_kick_signal_this_cycle: MainOutput<bool>,
    pub detected_free_kick_kicking_team: MainOutput<Option<Team>>,
}

impl FreeKickSignalFilter {
    pub fn new(context: CreationContext) -> Result<Self> {
        Ok(Self {
            detected_free_kick_detections_queue: VecDeque::with_capacity(
                *context.referee_pose_queue_length,
            ),
            free_kick_signal_detection_times: Default::default(),
            has_sent_free_kick_signal_message: false,
        })
    }

    pub fn cycle(
        &mut self,
        mut context: CycleContext<impl NetworkInterface>,
    ) -> Result<MainOutputs> {
        let free_kick_signal_detection_result = self.update(&context)?;

        context
            .free_kick_detection_times
            .fill_if_subscribed(|| self.free_kick_signal_detection_times);

        context
            .free_kick_detections_queue
            .fill_if_subscribed(|| self.detected_free_kick_detections_queue.clone());

        Ok(MainOutputs {
            detected_free_kick_kicking_team: free_kick_signal_detection_result
                .detected_free_kick_kicking_team
                .into(),
            did_detect_any_free_kick_signal_this_cycle: free_kick_signal_detection_result
                .did_detect_any_free_kick_signal_this_cycle
                .into(),
        })
    }

    fn update(
        &mut self,
        context: &CycleContext<impl NetworkInterface>,
    ) -> Result<FreeKickSignalDetectionResult> {
        if !matches!(
            context.game_controller_state.sub_state,
            Some(SubState::KickIn) | Some(SubState::PushingFreeKick)
        ) {
            self.detected_free_kick_detections_queue = Default::default();
            self.free_kick_signal_detection_times = Default::default();
            self.has_sent_free_kick_signal_message = false;

            return Ok(FreeKickSignalDetectionResult {
                did_detect_any_free_kick_signal_this_cycle: false,
                detected_free_kick_kicking_team: None,
            });
        }

        let time_tagged_persistent_messages =
            unpack_incoming_messages(&context.network_message.persistent);

        for (time, message) in time_tagged_persistent_messages {
            self.free_kick_signal_detection_times[message.player_number] = message
                .kicking_team
                .map(|kicking_team| TimeTaggedKickingTeamDetections {
                    time,
                    detected_kicking_team: kicking_team,
                });
        }

        let own_detected_pose_times: BTreeMap<SystemTime, Option<PoseKind>> =
            unpack_own_detections(&context.referee_pose_kind.persistent);

        let mut did_detect_any_free_kick_signal_this_cycle = false;

        for (_, detection) in own_detected_pose_times {
            let detected_kicking_team = kicking_team_from_free_kick_signal_detection(
                detection,
                context.game_controller_state.global_field_side,
            );
            if let Some(detected_kicking_team) = detected_kicking_team {
                self.detected_free_kick_detections_queue
                    .push_front(detected_kicking_team);
                did_detect_any_free_kick_signal_this_cycle = true
            } else {
                continue;
            }
        }

        self.detected_free_kick_detections_queue
            .truncate(*context.referee_pose_queue_length);

        let (own_detected_kicking_team, number_of_detections) =
            most_detections(self.detected_free_kick_detections_queue.clone().into());

        if number_of_detections >= *context.minimum_number_poses_before_message {
            self.free_kick_signal_detection_times[*context.player_number] =
                Some(TimeTaggedKickingTeamDetections {
                    time: context.cycle_time.start_time,
                    detected_kicking_team: own_detected_kicking_team,
                });
            if self.has_sent_free_kick_signal_message {
                send_own_detection_message(
                    context.hardware_interface.clone(),
                    *context.player_number,
                    Some(own_detected_kicking_team),
                )?;
                self.has_sent_free_kick_signal_message = true;
            }
        }

        let majority_voted_kicking_team_detection = majority_vote_free_kick_signal(
            self.free_kick_signal_detection_times,
            context.cycle_time.start_time,
            *context.initial_message_grace_period,
            *context.minimum_free_kick_signal_detections,
        );

        Ok(FreeKickSignalDetectionResult {
            did_detect_any_free_kick_signal_this_cycle,
            detected_free_kick_kicking_team: majority_voted_kicking_team_detection,
        })
    }
}

fn most_detections(detections: Vec<Team>) -> (Team, usize) {
    let own_detections_hulks: Vec<Team> = detections
        .iter()
        .cloned()
        .filter(|kicking_team| *kicking_team == Team::Hulks)
        .collect();
    let own_detections_opponent: Vec<Team> = detections
        .iter()
        .cloned()
        .filter(|kicking_team| *kicking_team == Team::Opponent)
        .collect();

    if own_detections_hulks.len() > own_detections_opponent.len() {
        (Team::Hulks, own_detections_hulks.len())
    } else {
        (Team::Opponent, own_detections_opponent.len())
    }
}

fn kicking_team_from_free_kick_signal_detection(
    free_kick_signal_pose: Option<PoseKind>,
    global_field_side: GlobalFieldSide,
) -> Option<Team> {
    match (global_field_side, free_kick_signal_pose) {
        (
            GlobalFieldSide::Home,
            Some(PoseKind::FreeKick {
                global_field_side: GlobalFieldSide::Away,
            }),
        ) => Some(Team::Hulks),
        (
            GlobalFieldSide::Home,
            Some(PoseKind::FreeKick {
                global_field_side: GlobalFieldSide::Home,
            }),
        ) => Some(Team::Opponent),
        (
            GlobalFieldSide::Away,
            Some(PoseKind::FreeKick {
                global_field_side: GlobalFieldSide::Away,
            }),
        ) => Some(Team::Opponent),
        (
            GlobalFieldSide::Away,
            Some(PoseKind::FreeKick {
                global_field_side: GlobalFieldSide::Home,
            }),
        ) => Some(Team::Hulks),
        _ => None,
    }
}

fn majority_vote_free_kick_signal(
    free_kick_signal_detection_times: Players<Option<TimeTaggedKickingTeamDetections>>,
    cycle_start_time: SystemTime,
    initial_message_grace_period: Duration,
    minimum_free_kick_signal_detections: usize,
) -> Option<Team> {
    let still_valid_detections = free_kick_signal_detection_times
        .iter()
        .filter_map(|(_, time_tagged_detection)| match time_tagged_detection {
            Some(TimeTaggedKickingTeamDetections {
                time,
                detected_kicking_team,
            }) if is_in_grace_period(cycle_start_time, *time, initial_message_grace_period) => {
                Some(*detected_kicking_team)
            }
            _ => None,
        })
        .collect();

    let (majority_voted_kicking_team, number_of_detections) =
        most_detections(still_valid_detections);
    if number_of_detections >= minimum_free_kick_signal_detections {
        Some(majority_voted_kicking_team)
    } else {
        None
    }
}

fn is_in_grace_period(
    cycle_start_time: SystemTime,
    earlier_time: SystemTime,
    grace_period: Duration,
) -> bool {
    cycle_start_time
        .duration_since(earlier_time)
        .expect("Time ran backwards")
        < grace_period
}

fn unpack_incoming_messages(
    messages: &BTreeMap<SystemTime, Vec<Option<&IncomingMessage>>>,
) -> BTreeMap<SystemTime, VisualRefereeMessage> {
    messages
        .iter()
        .flat_map(|(time, messages)| messages.iter().map(|message| (*time, message)))
        .filter_map(|(time, message)| match message {
            Some(IncomingMessage::Spl(HulkMessage::VisualReferee(message))) => {
                Some((time, *message))
            }
            _ => None,
        })
        .collect()
}

fn unpack_own_detections(
    detections: &BTreeMap<SystemTime, Vec<Option<&PoseKind>>>,
) -> BTreeMap<SystemTime, Option<PoseKind>> {
    detections
        .iter()
        .flat_map(|(time, pose_kinds)| {
            pose_kinds
                .iter()
                .map(|&pose_kind| (*time, pose_kind.cloned()))
        })
        .collect()
}

fn send_own_detection_message<T: NetworkInterface>(
    hardware_interface: Arc<T>,
    player_number: PlayerNumber,
    kicking_team: Option<Team>,
) -> Result<()> {
    hardware_interface.write_to_network(OutgoingMessage::Spl(HulkMessage::VisualReferee(
        VisualRefereeMessage {
            player_number,
            kicking_team,
        },
    )))
}
