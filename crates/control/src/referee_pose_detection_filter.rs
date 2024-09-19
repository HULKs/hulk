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
use spl_network_messages::{HulkMessage, VisualRefereeMessage};
use types::{
    cycle_time::CycleTime,
    messages::{IncomingMessage, OutgoingMessage},
    players::Players,
    pose_detection::VisualRefereeState,
    pose_kinds::PoseKind,
};

#[derive(Deserialize, Serialize)]
pub struct RefereePoseDetectionFilter {
    detection_times: Players<Option<SystemTime>>,
    detected_above_arm_poses_queue: VecDeque<bool>,
    motion_in_standby_count: usize,
    visual_referee_state: VisualRefereeState,
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
        Parameter<Duration, "referee_pose_detection_filter.initial_message_grace_period">,
    minimum_above_head_arms_detections:
        Parameter<usize, "referee_pose_detection_filter.minimum_above_head_arms_detections">,
    jersey_number: Parameter<usize, "jersey_number">,
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
    pub majority_vote_is_referee_ready_pose_detected: MainOutput<bool>,
    pub is_referee_ready_pose_detected: MainOutput<bool>,
    pub did_detect_any_referee_this_cycle: MainOutput<bool>,
}

impl RefereePoseDetectionFilter {
    pub fn new(context: CreationContext) -> Result<Self> {
        Ok(Self {
            visual_referee_state: VisualRefereeState::WaitingForDetections,
            detection_times: Default::default(),
            motion_in_standby_count: 0,
            detected_above_arm_poses_queue: VecDeque::with_capacity(
                *context.referee_pose_queue_length,
            ),
        })
    }

    pub fn cycle(
        &mut self,
        mut context: CycleContext<impl NetworkInterface>,
    ) -> Result<MainOutputs> {
        let cycle_start_time = context.cycle_time.start_time;

        let (is_referee_ready_pose_detected, did_detect_any_referee_this_cycle) =
            self.update(&context)?;

        let majority_vote_is_referee_ready_pose_detected = decide(
            &self.detection_times,
            cycle_start_time,
            *context.initial_message_grace_period,
            *context.minimum_above_head_arms_detections,
        );

        context
            .player_referee_detection_times
            .fill_if_subscribed(|| self.detection_times.clone());

        context
            .referee_pose_queue
            .fill_if_subscribed(|| self.detected_above_arm_poses_queue.clone());

        Ok(MainOutputs {
            majority_vote_is_referee_ready_pose_detected:
                majority_vote_is_referee_ready_pose_detected.into(),
            is_referee_ready_pose_detected: is_referee_ready_pose_detected.into(),
            did_detect_any_referee_this_cycle: did_detect_any_referee_this_cycle.into(),
        })
    }

    fn update(&mut self, context: &CycleContext<impl NetworkInterface>) -> Result<(bool, bool)> {
        let time_tagged_persistent_messages =
            unpack_message_tree(&context.network_message.persistent);

        for (time, message) in time_tagged_persistent_messages {
            self.detection_times[message.jersey_number] = Some(time);
        }
        let own_detected_pose_times =
            unpack_own_detection_tree(&context.referee_pose_kind.persistent);
        let mut did_detect_any_referee_this_cycle = false;

        for (_, detection) in own_detected_pose_times {
            let detected_visual_referee =
                detection.map_or(false, |pose_kind| pose_kind == PoseKind::AboveHeadArms);
            self.detected_above_arm_poses_queue
                .push_front(detected_visual_referee);
            did_detect_any_referee_this_cycle |= detected_visual_referee
        }

        self.detected_above_arm_poses_queue
            .truncate(*context.referee_pose_queue_length);

        let detected_referee_pose_count = self
            .detected_above_arm_poses_queue
            .iter()
            .filter(|x| **x)
            .count();

        if detected_referee_pose_count >= *context.minimum_number_poses_before_message {
            self.detection_times[*context.jersey_number] = Some(context.cycle_time.start_time);

            send_own_detection_message(context.hardware_interface.clone(), *context.jersey_number)?;
        }

        Ok((
            detected_referee_pose_count >= *context.minimum_number_poses_before_message,
            did_detect_any_referee_this_cycle,
        ))
    }
}

fn decide(
    pose_detection_times: &Players<Option<SystemTime>>,
    cycle_start_time: SystemTime,
    initial_message_grace_period: Duration,
    minimum_above_head_arms_detections: usize,
) -> bool {
    let detected_above_head_arms_poses = pose_detection_times
        .inner
        .iter()
        .filter(|detection_time| match detection_time {
            Some(detection_time) => is_in_grace_period(
                cycle_start_time,
                *detection_time,
                initial_message_grace_period,
            ),
            None => false,
        })
        .count();
    detected_above_head_arms_poses >= minimum_above_head_arms_detections
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
    jersey_number: usize,
) -> Result<()> {
    hardware_interface.write_to_network(OutgoingMessage::Spl(HulkMessage::VisualReferee(
        VisualRefereeMessage { jersey_number },
    )))
}
