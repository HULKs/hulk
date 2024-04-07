use std::time::{Duration, SystemTime};

use color_eyre::Result;
use context_attribute::context;
use framework::{MainOutput, PerceptionInput};
use serde::{Deserialize, Serialize};
use spl_network_messages::PlayerNumber;
use types::{
    cycle_time::CycleTime, messages::IncomingMessage,
    parameters::RefereePoseDetectionFilterParameters, pose_types::PoseType,
};

#[derive(Deserialize, Serialize)]
pub struct RefereePoseDetectionFilter {
    pose_detection_times: Vec<Option<SystemTime>>,
}

#[context]
pub struct CreationContext {}

#[context]
pub struct CycleContext {
    network_message: PerceptionInput<IncomingMessage, "SplNetwork", "message">,
    detected_referee_pose_type:
        PerceptionInput<PoseType, "DetectionTop", "detected_referee_pose_type">,
    cycle_time: Input<CycleTime, "cycle_time">,

    parameters: Parameter<RefereePoseDetectionFilterParameters, "referee_pose_detection_filter">,
    player_number: Parameter<PlayerNumber, "player_number">,
}

#[context]
#[derive(Default)]
pub struct MainOutputs {
    pub ready_to_initial_trigger: MainOutput<bool>,
}

impl RefereePoseDetectionFilter {
    pub fn new(_context: CreationContext) -> Result<Self> {
        Ok(Self {
            pose_detection_times: vec![None; 7],
        })
    }

    pub fn cycle(&mut self, context: CycleContext) -> Result<MainOutputs> {
        let cycle_start_time = context.cycle_time.start_time;

        let pose_detection_times = self.update(
            context.detected_referee_pose_type,
            *context.player_number,
            context.network_message,
        );

        let ready_to_initial_trigger =
            self.decide(pose_detection_times, cycle_start_time, context.parameters);
        Ok(MainOutputs {
            ready_to_initial_trigger: ready_to_initial_trigger.into(),
        })
    }

    fn update(
        &mut self,
        detected_referee_pose_type: PerceptionInput<Vec<&PoseType>>,
        player_number: PlayerNumber,
        spl_messages: PerceptionInput<Vec<&IncomingMessage>>,
    ) -> Vec<Option<SystemTime>> {
        let persistent_messages: Vec<_> = spl_messages
            .persistent
            .iter()
            .flat_map(|(time, messages)| messages.iter().map(|message| (*time, message)))
            .filter_map(|(time, message)| match message {
                IncomingMessage::GameController(_) => None,
                IncomingMessage::Spl(message) => Some((time, message)),
            })
            .collect();

        for (time, message) in persistent_messages {
            if message.over_arms_pose_detected {
                self.pose_detection_times[message.player_number as usize] = Some(time);
            }
        }

        let persistent_own_detected_pose_time = detected_referee_pose_type
            .persistent
            .iter()
            .flat_map(|(time, pose_types)| pose_types.iter().map(|pose_type| (*time, pose_type)))
            .filter_map(|(time, pose_type)| match pose_type {
                PoseType::OverheadArms => Some(time),
                _ => None,
            })
            .last();

        self.pose_detection_times[player_number as usize] = persistent_own_detected_pose_time;

        let mut temporary_pose_detection_times = self.pose_detection_times.clone();

        let temporary_messages: Vec<_> = spl_messages
            .temporary
            .iter()
            .flat_map(|(time, messages)| messages.iter().map(|message| (*time, message)))
            .filter_map(|(time, message)| match message {
                IncomingMessage::GameController(_) => None,
                IncomingMessage::Spl(message) => Some((time, message)),
            })
            .collect();

        for (time, message) in temporary_messages {
            if message.over_arms_pose_detected {
                temporary_pose_detection_times[message.player_number as usize] = Some(time);
            }
        }

        let temporary_own_detected_pose_time = detected_referee_pose_type
            .temporary
            .iter()
            .flat_map(|(time, pose_types)| pose_types.iter().map(|pose_type| (*time, pose_type)))
            .filter_map(|(time, pose_type)| match pose_type {
                PoseType::OverheadArms => Some(time),
                _ => None,
            })
            .last();

        temporary_pose_detection_times[player_number as usize] = temporary_own_detected_pose_time;

        dbg!(&temporary_pose_detection_times);
        temporary_pose_detection_times
    }

    fn decide(
        &mut self,
        pose_detection_times: Vec<Option<SystemTime>>,
        cycle_start_time: SystemTime,
        parameters: &RefereePoseDetectionFilterParameters,
    ) -> bool {
        let detected_over_head_arms_poses = pose_detection_times
            .iter()
            .filter(|detection_time| match detection_time {
                Some(detection_time) => Self::in_grace_period(
                    cycle_start_time,
                    *detection_time,
                    parameters.initial_message_grace_period,
                ),
                None => false,
            })
            .count();
        dbg!(detected_over_head_arms_poses);
        detected_over_head_arms_poses >= parameters.minimum_over_head_arms_detections
    }

    fn in_grace_period(
        cycle_start_time: SystemTime,
        earlier_time: SystemTime,
        grace_period: Duration,
    ) -> bool {
        cycle_start_time
            .duration_since(earlier_time)
            .expect("Time ran backwards")
            < grace_period
    }
}
