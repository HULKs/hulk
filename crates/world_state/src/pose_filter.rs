use color_eyre::Result;
use serde::{Deserialize, Serialize};

use context_attribute::context;
use framework::{MainOutput, PerceptionInput};
use types::{
    object_detection::YOLOObjectLabel,
    parameters::PoseFilteringParameters,
    pose_detection::{OVERALL_KEYPOINT_INDEX_MASK, Pose, VISUAL_REFEREE_KEYPOINT_INDEX_MASK},
};

#[derive(Deserialize, Serialize)]
pub struct PoseFilter {
    pub last_detected_poses: Vec<Pose<YOLOObjectLabel>>,
}

#[context]
pub struct CreationContext {}

#[context]
pub struct CycleContext {
    detected_poses: PerceptionInput<Vec<Pose<YOLOObjectLabel>>, "Hydra", "detected_poses">,

    parameters: Parameter<PoseFilteringParameters, "pose_filtering">,
}

#[context]
#[derive(Default)]
pub struct MainOutputs {
    pub accepted_human_poses: MainOutput<Vec<Pose<YOLOObjectLabel>>>,
    pub rejected_human_poses: MainOutput<Vec<Pose<YOLOObjectLabel>>>,
}

impl PoseFilter {
    pub fn new(_creation_context: CreationContext) -> Result<Self> {
        Ok(PoseFilter {
            last_detected_poses: Vec::new(),
        })
    }

    pub fn cycle(&mut self, context: CycleContext) -> Result<MainOutputs> {
        let detected_poses = Self::latest_detected_poses(self, &context);

        let (accepted_human_poses, rejected_human_poses): (
            Vec<Pose<YOLOObjectLabel>>,
            Vec<Pose<YOLOObjectLabel>>,
        ) = detected_poses.iter().copied().partition(|pose| {
            filter_poses_by_overall_confidence(
                pose,
                context.parameters.minimum_overall_keypoint_confidence,
            ) && filter_poses_by_visual_referee_confidence(
                pose,
                context
                    .parameters
                    .minimum_visual_referee_keypoint_confidence,
            ) && pose.object.label == YOLOObjectLabel::Person
        });

        Ok(MainOutputs {
            accepted_human_poses: accepted_human_poses.into(),
            rejected_human_poses: rejected_human_poses.into(),
        })
    }

    fn latest_detected_poses(&mut self, context: &CycleContext) -> Vec<Pose<YOLOObjectLabel>> {
        let detected_poses = context
            .detected_poses
            .persistent
            .iter()
            .chain(context.detected_poses.temporary.iter())
            .flat_map(|(_timestamp, detected_poses)| detected_poses.iter().cloned().cloned())
            .next_back()
            .unwrap_or(self.last_detected_poses.clone());
        self.last_detected_poses = detected_poses.clone();
        detected_poses
    }
}

fn filter_poses_by_overall_confidence(
    pose: &Pose<YOLOObjectLabel>,
    minimum_overall_keypoint_confidence: f32,
) -> bool {
    let visual_referee_keypoint_indices = OVERALL_KEYPOINT_INDEX_MASK;
    visual_referee_keypoint_indices
        .into_iter()
        .all(|index| pose.keypoints[index].confidence > minimum_overall_keypoint_confidence)
}

fn filter_poses_by_visual_referee_confidence(
    pose: &Pose<YOLOObjectLabel>,
    minimum_visual_referee_keypoint_confidence: f32,
) -> bool {
    let visual_referee_keypoint_indices = VISUAL_REFEREE_KEYPOINT_INDEX_MASK;
    visual_referee_keypoint_indices
        .into_iter()
        .all(|index| pose.keypoints[index].confidence > minimum_visual_referee_keypoint_confidence)
}
