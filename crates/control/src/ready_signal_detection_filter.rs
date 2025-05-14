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
use spl_network_messages::{GameState, HulkMessage, PlayerNumber, VisualRefereeMessage};
use types::{
    cycle_time::CycleTime,
    game_controller_state::GameControllerState,
    messages::{IncomingMessage, OutgoingMessage},
    parameters::SplNetworkParameters,
    players::Players,
    pose_detection::{ReadySignalDetectionFeedback, ReadySignalState},
    pose_kinds::PoseKind,
};

#[derive(Deserialize, Serialize)]
pub struct ReadySignalDetectionFilter {
    detection_times: Players<Option<SystemTime>>,
    detected_ready_signal_queue: VecDeque<bool>,
    motion_in_standby_count: usize,
    last_time_message_sent: Option<SystemTime>,
    ready_signal_state: ReadySignalState,
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

    cycle_time: Input<CycleTime, "cycle_time">,
    remaining_amount_of_messages:
        Input<Option<u16>, "game_controller_state?.hulks_team.remaining_amount_of_messages">,
    game_controller_state: RequiredInput<Option<GameControllerState>, "game_controller_state?">,

    initial_message_grace_period:
        Parameter<Duration, "referee_pose_detection_filter.initial_message_grace_period">,
    message_interval: Parameter<Duration, "referee_pose_detection_filter.message_interval">,
    minimum_ready_signal_detections:
        Parameter<usize, "ready_signal_detection_filter.minimum_ready_signal_detections">,
    player_number: Parameter<PlayerNumber, "player_number">,
    referee_pose_queue_length: Parameter<usize, "pose_detection.referee_pose_queue_length">,
    minimum_number_poses_before_message:
        Parameter<usize, "pose_detection.minimum_number_poses_before_message">,
    spl_network_parameters: Parameter<SplNetworkParameters, "spl_network">,

    player_referee_detection_times:
        AdditionalOutput<Players<Option<SystemTime>>, "player_referee_detection_times">,
    referee_pose_queue: AdditionalOutput<VecDeque<bool>, "referee_pose_queue">,
}

#[context]
#[derive(Default)]
pub struct MainOutputs {
    pub is_majority_vote_referee_ready_pose_detected: MainOutput<bool>,
    pub is_own_referee_ready_pose_detected: MainOutput<bool>,
    pub did_detect_any_ready_signal_this_cycle: MainOutput<bool>,
}

impl ReadySignalDetectionFilter {
    pub fn new(context: CreationContext) -> Result<Self> {
        Ok(Self {
            detection_times: Default::default(),
            detected_ready_signal_queue: VecDeque::with_capacity(
                *context.referee_pose_queue_length,
            ),
            motion_in_standby_count: 0,
            last_time_message_sent: None,
            ready_signal_state: ReadySignalState::WaitingForDetections,
        })
    }

    pub fn cycle(
        &mut self,
        mut context: CycleContext<impl NetworkInterface>,
    ) -> Result<MainOutputs> {
        if !matches!(
            context.game_controller_state.game_state,
            GameState::Standby { .. }
        ) {
            self.detected_ready_signal_queue = Default::default();
            self.motion_in_standby_count = Default::default();
            self.detection_times = Default::default();
            self.ready_signal_state = Default::default();

            return Ok(MainOutputs {
                is_majority_vote_referee_ready_pose_detected: false.into(),
                is_own_referee_ready_pose_detected: false.into(),
                did_detect_any_ready_signal_this_cycle: false.into(),
            });
        }

        let cycle_start_time = context.cycle_time.start_time;

        let ready_signal_detection_feedback = self.update_own_detections(&context)?;

        let is_majority_vote_referee_ready_pose_detected = majority_vote_ready_signal(
            self.detection_times,
            cycle_start_time,
            *context.initial_message_grace_period,
            *context.minimum_ready_signal_detections,
        );

        context
            .player_referee_detection_times
            .fill_if_subscribed(|| self.detection_times);

        context
            .referee_pose_queue
            .fill_if_subscribed(|| self.detected_ready_signal_queue.clone());

        Ok(MainOutputs {
            is_majority_vote_referee_ready_pose_detected:
                is_majority_vote_referee_ready_pose_detected.into(),
            is_own_referee_ready_pose_detected: ready_signal_detection_feedback
                .is_referee_ready_pose_detected
                .into(),
            did_detect_any_ready_signal_this_cycle: ready_signal_detection_feedback
                .did_detect_any_ready_signal_this_cycle
                .into(),
        })
    }

    fn update_own_detections(
        &mut self,
        context: &CycleContext<impl NetworkInterface>,
    ) -> Result<ReadySignalDetectionFeedback> {
        let time_tagged_persistent_messages =
            unpack_message_tree(&context.network_message.persistent);

        for (time, message) in time_tagged_persistent_messages {
            self.detection_times[message.player_number] = Some(time);
        }
        let own_detected_pose_times: BTreeMap<SystemTime, Option<PoseKind>> =
            unpack_own_detections(&context.referee_pose_kind.persistent);

        let ready_signal_detection_feedback =
            Self::own_ready_signal_detection_evaluation(self, context, own_detected_pose_times)?;

        Ok(ready_signal_detection_feedback)
    }

    fn own_ready_signal_detection_evaluation(
        &mut self,
        context: &CycleContext<impl NetworkInterface>,
        own_detected_pose_times: BTreeMap<SystemTime, Option<PoseKind>>,
    ) -> Result<ReadySignalDetectionFeedback> {
        let mut did_detect_any_ready_signal_this_cycle = false;

        for (_, detection) in own_detected_pose_times {
            let detected_visual_referee = detection == Some(PoseKind::Ready);
            self.detected_ready_signal_queue
                .push_front(detected_visual_referee);
            did_detect_any_ready_signal_this_cycle |= detected_visual_referee
        }

        self.detected_ready_signal_queue
            .truncate(*context.referee_pose_queue_length);

        let detected_referee_pose_count = self
            .detected_ready_signal_queue
            .iter()
            .filter(|x| **x)
            .count();
        if detected_referee_pose_count >= *context.minimum_number_poses_before_message {
            let now = context.cycle_time.start_time;
            self.detection_times[*context.player_number] = Some(now);
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
                )?;
                self.last_time_message_sent = Some(now);
            }
        }

        Ok(ReadySignalDetectionFeedback {
            is_referee_ready_pose_detected: detected_referee_pose_count
                >= *context.minimum_number_poses_before_message,
            did_detect_any_ready_signal_this_cycle,
        })
    }
}

fn majority_vote_ready_signal(
    ready_signal_detection_times: Players<Option<SystemTime>>,
    cycle_start_time: SystemTime,
    initial_message_grace_period: Duration,
    minimum_ready_signal_detections: usize,
) -> bool {
    let detected_ready_signal_detections = ready_signal_detection_times
        .iter()
        .filter(|(_, detection_time)| match detection_time {
            Some(detection_time) => is_in_grace_period(
                cycle_start_time,
                *detection_time,
                initial_message_grace_period,
            ),
            None => false,
        })
        .count();
    detected_ready_signal_detections >= minimum_ready_signal_detections
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

fn unpack_message_tree(
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
) -> Result<()> {
    hardware_interface.write_to_network(OutgoingMessage::Spl(HulkMessage::VisualReferee(
        VisualRefereeMessage { player_number },
    )))
}
