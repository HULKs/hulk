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
    pose_detection::{ReadySignalDetectionResult, ReadySignalState},
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

    game_controller_state: RequiredInput<Option<GameControllerState>, "game_controller_state?">,

    cycle_time: Input<CycleTime, "cycle_time">,
    remaining_amount_of_messages:
        Input<Option<u16>, "game_controller_state?.hulks_team.remaining_amount_of_messages">,

    initial_message_grace_period:
        Parameter<Duration, "referee_pose_detection_filter.initial_message_grace_period">,
    minimum_ready_signal_detections:
        Parameter<usize, "ready_signal_detection_filter.minimum_ready_signal_detections">,
    player_number: Parameter<PlayerNumber, "player_number">,
    referee_pose_queue_length: Parameter<usize, "pose_detection.referee_pose_queue_length">,
    minimum_number_poses_before_message:
        Parameter<usize, "pose_detection.minimum_number_poses_before_message">,
    message_interval: Parameter<Duration, "referee_pose_detection_filter.message_interval">,
    spl_network_parameters: Parameter<SplNetworkParameters, "spl_network">,

    ready_signal_detection_times:
        AdditionalOutput<Players<Option<SystemTime>>, "player_referee_detection_times">,
    detected_ready_signal_queue: AdditionalOutput<VecDeque<bool>, "referee_pose_queue">,
}

#[context]
#[derive(Default)]
pub struct MainOutputs {
    pub ready_signal_detected: MainOutput<bool>,
    pub own_ready_signal_detection_result: MainOutput<ReadySignalDetectionResult>,
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
                ready_signal_detected: false.into(),
                own_ready_signal_detection_result: ReadySignalDetectionResult::default().into(),
            });
        }

        let own_ready_signal_detection_result = self.update_own_detections(&context)?;

        self.update_other_detections(&context);

        let ready_signal_detected = majority_vote_ready_signal(
            self.detection_times,
            context.cycle_time.start_time,
            *context.initial_message_grace_period,
            *context.minimum_ready_signal_detections,
        );

        context
            .ready_signal_detection_times
            .fill_if_subscribed(|| self.detection_times);

        context
            .detected_ready_signal_queue
            .fill_if_subscribed(|| self.detected_ready_signal_queue.clone());

        Ok(MainOutputs {
            ready_signal_detected: ready_signal_detected.into(),
            own_ready_signal_detection_result: own_ready_signal_detection_result.into(),
        })
    }

    fn update_own_detections(
        &mut self,
        context: &CycleContext<impl NetworkInterface>,
    ) -> Result<ReadySignalDetectionResult> {
        let own_detected_pose_times: BTreeMap<SystemTime, Option<PoseKind>> =
            unpack_own_detections(&context.referee_pose_kind.persistent);

        let mut did_detect_any_ready_pose_this_cycle = false;

        for (_, detection) in own_detected_pose_times {
            let detected_visual_referee = detection == Some(PoseKind::Ready);
            self.detected_ready_signal_queue
                .push_front(detected_visual_referee);
            did_detect_any_ready_pose_this_cycle |= detected_visual_referee
        }

        self.detected_ready_signal_queue
            .truncate(*context.referee_pose_queue_length);

        let detected_referee_pose_count = self
            .detected_ready_signal_queue
            .iter()
            .filter(|x| **x)
            .count();

        let detected_own_ready_signal =
            detected_referee_pose_count >= *context.minimum_number_poses_before_message;

        if detected_own_ready_signal {
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

        Ok(ReadySignalDetectionResult {
            detected_own_ready_signal,
            did_detect_any_ready_pose_this_cycle,
        })
    }

    fn update_other_detections(&mut self, context: &CycleContext<impl NetworkInterface>) {
        let time_tagged_persistent_messages =
            unpack_other_detections(&context.network_message.persistent);

        for (time, message) in time_tagged_persistent_messages {
            self.detection_times[message.player_number] = Some(time);
        }
    }
}

fn majority_vote_ready_signal(
    ready_signal_detection_times: Players<Option<SystemTime>>,
    cycle_start_time: SystemTime,
    initial_message_grace_period: Duration,
    minimum_ready_signal_detections: usize,
) -> bool {
    let ready_signal_detections = ready_signal_detection_times
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
    ready_signal_detections >= minimum_ready_signal_detections
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
) -> Result<()> {
    hardware_interface.write_to_network(OutgoingMessage::Spl(HulkMessage::VisualReferee(
        VisualRefereeMessage {
            player_number,
            kicking_team: None,
        },
    )))
}
