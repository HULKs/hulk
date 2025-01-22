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
use spl_network_messages::{HulkMessage, PlayerNumber, VisualRefereeMessage};
use types::{
    cycle_time::CycleTime,
    messages::{IncomingMessage, OutgoingMessage},
    players::Players,
    pose_detection::{ReadySignalDetectionFeedback, ReadySignalState},
    pose_kinds::PoseKind,
};

#[derive(Deserialize, Serialize)]
pub struct ReadySignalDetectionFilter {
    ready_signal_detection_times: Players<Option<SystemTime>>,
    detected_ready_signal_queue: VecDeque<bool>,
    motion_in_standby_count: usize,
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

    initial_message_grace_period:
        Parameter<Duration, "ready_signal_detection_filter.initial_message_grace_period">,
    minimum_above_head_arms_detections:
        Parameter<usize, "ready_signal_detection_filter.minimum_above_head_arms_detections">,
    player_number: Parameter<PlayerNumber, "player_number">,
    referee_pose_queue_length: Parameter<usize, "pose_detection.referee_pose_queue_length">,
    minimum_number_poses_before_message:
        Parameter<usize, "pose_detection.minimum_number_poses_before_message">,

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
            ready_signal_detection_times: Default::default(),
            detected_ready_signal_queue: VecDeque::with_capacity(
                *context.referee_pose_queue_length,
            ),
            motion_in_standby_count: 0,
            ready_signal_state: ReadySignalState::WaitingForDetections,
        })
    }

    pub fn cycle(
        &mut self,
        mut context: CycleContext<impl NetworkInterface>,
    ) -> Result<MainOutputs> {
        let cycle_start_time = context.cycle_time.start_time;

        let ready_signal_detection_feedback = self.update_own_detections(&context)?;

        let is_majority_vote_referee_ready_pose_detected = majority_vote_ready_signal(
            self.ready_signal_detection_times,
            cycle_start_time,
            *context.initial_message_grace_period,
            *context.minimum_above_head_arms_detections,
        );

        context
            .player_referee_detection_times
            .fill_if_subscribed(|| self.ready_signal_detection_times);

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
            self.ready_signal_detection_times[message.player_number] = Some(time);
        }
        let own_detected_pose_times: BTreeMap<SystemTime, Option<PoseKind>> =
            unpack_own_detection_tree(&context.referee_pose_kind.persistent);

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
            let detected_visual_referee =
                detection.map_or(false, |pose_kind| pose_kind == PoseKind::AboveHeadArms);
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
            self.ready_signal_detection_times[*context.player_number] =
                Some(context.cycle_time.start_time);

            send_own_detection_message(context.hardware_interface.clone(), *context.player_number)?;
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
    minimum_above_head_arms_detections: usize,
) -> bool {
    let detected_ready_signal_poses = ready_signal_detection_times
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
    detected_ready_signal_poses >= minimum_above_head_arms_detections
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

fn unpack_own_detection_tree(
    pose_kind_tree: &BTreeMap<SystemTime, Vec<Option<&PoseKind>>>,
) -> BTreeMap<SystemTime, Option<PoseKind>> {
    pose_kind_tree
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
