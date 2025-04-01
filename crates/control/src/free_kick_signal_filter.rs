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
use spl_network_messages::{
    GameState, HulkMessage, PlayerNumber, SubState, Team, VisualRefereeMessage,
};
use types::{
    cycle_time::CycleTime,
    field_dimensions::GlobalFieldSide,
    game_controller_state::GameControllerState,
    messages::{IncomingMessage, OutgoingMessage},
    parameters::SplNetworkParameters,
    players::Players,
    pose_detection::{FreeKickSignalDetectionResult, TimeTaggedKickingTeamDetections},
    pose_kinds::PoseKind,
};

#[derive(Deserialize, Serialize)]
pub struct FreeKickSignalFilter {
    detection_times: Players<Option<TimeTaggedKickingTeamDetections>>,
    detected_free_kick_signal_queue: VecDeque<Team>,
    last_time_message_sent: Option<SystemTime>,
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
    remaining_amount_of_messages:
        Input<Option<u16>, "game_controller_state?.hulks_team.remaining_amount_of_messages">,

    visual_referee_message_grace_period: Parameter<
        Duration,
        "free_kick_signal_detection_filter.visual_referee_message_grace_period",
    >,
    minimum_free_kick_signal_detections:
        Parameter<usize, "free_kick_signal_detection_filter.minimum_free_kick_signal_detections">,
    player_number: Parameter<PlayerNumber, "player_number">,
    referee_pose_queue_length: Parameter<usize, "pose_detection.referee_pose_queue_length">,
    minimum_number_poses_before_message:
        Parameter<usize, "pose_detection.minimum_number_poses_before_message">,
    message_interval: Parameter<Duration, "referee_pose_detection_filter.message_interval">,
    spl_network_parameters: Parameter<SplNetworkParameters, "spl_network">,

    free_kick_detection_times: AdditionalOutput<
        Players<Option<TimeTaggedKickingTeamDetections>>,
        "free_kick_detection_times",
    >,
    detected_free_kick_signal_queue: AdditionalOutput<VecDeque<Team>, "free_kick_detections_queue">,
}

#[context]
#[derive(Default)]
pub struct MainOutputs {
    pub detected_free_kick_kicking_team: MainOutput<Option<Team>>,
    pub own_free_kick_signal_detection_result: MainOutput<FreeKickSignalDetectionResult>,
}

impl FreeKickSignalFilter {
    pub fn new(context: CreationContext) -> Result<Self> {
        Ok(Self {
            detected_free_kick_signal_queue: VecDeque::with_capacity(
                *context.referee_pose_queue_length,
            ),
            detection_times: Default::default(),
            last_time_message_sent: None,
        })
    }

    pub fn cycle(
        &mut self,
        mut context: CycleContext<impl NetworkInterface>,
    ) -> Result<MainOutputs> {
        if !matches!(
            context.game_controller_state,
            GameControllerState {
                game_state: GameState::Playing,
                sub_state: Some(SubState::KickIn),
                ..
            }
        ) {
            self.detected_free_kick_signal_queue =
                VecDeque::with_capacity(*context.referee_pose_queue_length);
            self.detection_times = Default::default();
            self.last_time_message_sent = None;

            return Ok(MainOutputs {
                detected_free_kick_kicking_team: None.into(),
                own_free_kick_signal_detection_result: FreeKickSignalDetectionResult::default()
                    .into(),
            });
        }

        let own_free_kick_signal_detection_result = self.update_own_detections(&context)?;

        self.update_other_detections(&context);

        let majority_voted_kicking_team_detection = majority_vote_free_kick_signal(
            self.detection_times,
            context.cycle_time.start_time,
            *context.visual_referee_message_grace_period,
            *context.minimum_free_kick_signal_detections,
        );

        context
            .free_kick_detection_times
            .fill_if_subscribed(|| self.detection_times);

        context
            .detected_free_kick_signal_queue
            .fill_if_subscribed(|| self.detected_free_kick_signal_queue.clone());

        Ok(MainOutputs {
            detected_free_kick_kicking_team: majority_voted_kicking_team_detection.into(),
            own_free_kick_signal_detection_result: own_free_kick_signal_detection_result.into(),
        })
    }

    fn update_own_detections(
        &mut self,
        context: &CycleContext<impl NetworkInterface>,
    ) -> Result<FreeKickSignalDetectionResult> {
        let own_detected_pose_times: BTreeMap<SystemTime, Option<PoseKind>> =
            unpack_own_detections(&context.referee_pose_kind.persistent);

        let mut did_detect_any_free_kick_pose_this_cycle = false;

        for (_, detection) in own_detected_pose_times {
            let detected_kicking_team = kicking_team_from_free_kick_pose(
                detection,
                context.game_controller_state.global_field_side,
            );
            if let Some(detected_kicking_team) = detected_kicking_team {
                self.detected_free_kick_signal_queue
                    .push_front(detected_kicking_team);
                did_detect_any_free_kick_pose_this_cycle = true
            } else {
                continue;
            }
        }

        self.detected_free_kick_signal_queue
            .truncate(*context.referee_pose_queue_length);

        let (own_detected_kicking_team, number_of_detections) =
            most_detections(self.detected_free_kick_signal_queue.make_contiguous());

        if number_of_detections >= *context.minimum_number_poses_before_message {
            let now = context.cycle_time.start_time;
            self.detection_times[*context.player_number] = Some(TimeTaggedKickingTeamDetections {
                time: context.cycle_time.start_time,
                detected_kicking_team: Some(own_detected_kicking_team),
            });
            if self.last_time_message_sent.as_ref().map_or(true, |time| {
                now.duration_since(*time).expect("Time ran backwards") >= *context.message_interval
            }) && context.remaining_amount_of_messages.is_some_and(
                |remaining_amount_of_messages| {
                    *remaining_amount_of_messages
                        > context
                            .spl_network_parameters
                            .remaining_amount_of_messages_to_stop_sending
                },
            ) {
                send_own_detection_message(
                    context.hardware_interface.clone(),
                    *context.player_number,
                    Some(own_detected_kicking_team),
                )?;
                self.last_time_message_sent = Some(now);
            }
        }

        Ok(FreeKickSignalDetectionResult {
            did_detect_any_free_kick_pose_this_cycle,
            own_detected_kicking_team: Some(own_detected_kicking_team),
        })
    }

    fn update_other_detections(&mut self, context: &CycleContext<impl NetworkInterface>) {
        let time_tagged_persistent_messages =
            unpack_other_detections(&context.network_message.persistent);

        for (time, message) in time_tagged_persistent_messages {
            self.detection_times[message.player_number] = Some(TimeTaggedKickingTeamDetections {
                time,
                detected_kicking_team: message.kicking_team,
            });
        }
    }
}

fn most_detections(detections: &[Team]) -> (Team, usize) {
    let number_of_own_detections_hulks = detections
        .iter()
        .copied()
        .filter(|kicking_team| *kicking_team == Team::Hulks)
        .count();
    let number_of_own_detections_opponent = detections
        .iter()
        .copied()
        .filter(|kicking_team| *kicking_team == Team::Opponent)
        .count();

    if number_of_own_detections_hulks > number_of_own_detections_opponent {
        (Team::Hulks, number_of_own_detections_hulks)
    } else {
        (Team::Opponent, number_of_own_detections_opponent)
    }
}

fn kicking_team_from_free_kick_pose(
    free_kick_signal_pose: Option<PoseKind>,
    global_field_side: GlobalFieldSide,
) -> Option<Team> {
    free_kick_signal_pose.map(|pose_kind| {
        if pose_kind == (PoseKind::FreeKick { global_field_side }) {
            Team::Opponent
        } else {
            Team::Hulks
        }
    })
}

fn majority_vote_free_kick_signal(
    free_kick_signal_detection_times: Players<Option<TimeTaggedKickingTeamDetections>>,
    cycle_start_time: SystemTime,
    visual_referee_message_grace_period: Duration,
    minimum_free_kick_signal_detections: usize,
) -> Option<Team> {
    let still_valid_detections: Vec<Team> = free_kick_signal_detection_times
        .iter()
        .filter_map(|(_, time_tagged_detection)| match time_tagged_detection {
            Some(TimeTaggedKickingTeamDetections {
                time,
                detected_kicking_team,
            }) if is_in_grace_period(
                cycle_start_time,
                *time,
                visual_referee_message_grace_period,
            ) =>
            {
                *detected_kicking_team
            }
            _ => None,
        })
        .collect();

    let (majority_voted_kicking_team, number_of_detections) =
        most_detections(&still_valid_detections);
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

fn unpack_other_detections(
    message_tree: &BTreeMap<SystemTime, Vec<Option<&IncomingMessage>>>,
) -> BTreeMap<SystemTime, VisualRefereeMessage> {
    message_tree
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
