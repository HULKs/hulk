use color_eyre::Result;
use serde::{Deserialize, Serialize};

use context_attribute::context;
use framework::MainOutput;
use types::pose_detection::{
    HumanPose, OVERALL_KEYPOINT_INDEX_MASK, VISUAL_REFEREE_KEYPOINT_INDEX_MASK,
};

#[derive(Deserialize, Serialize)]
pub struct PoseFilter {}

#[context]
pub struct CreationContext {}

#[context]
pub struct CycleContext {
    unfiltered_poses: Input<Vec<HumanPose>, "unfiltered_human_poses">,

    minimum_overall_keypoint_confidence:
        Parameter<f32, "pose_detection.minimum_overall_keypoint_confidence">,
    minimum_visual_referee_keypoint_confidence:
        Parameter<f32, "pose_detection.minimum_visual_referee_keypoint_confidence">,
}

#[context]
#[derive(Default)]
pub struct MainOutputs {
    pub accepted_human_poses: MainOutput<Vec<HumanPose>>,
    pub rejected_human_poses: MainOutput<Vec<HumanPose>>,
}

impl PoseFilter {
    pub fn new(_creation_context: CreationContext) -> Result<Self> {
        Ok(PoseFilter {})
    }

    pub fn cycle(&mut self, context: CycleContext) -> Result<MainOutputs> {
        let (accepted_human_poses, rejected_human_poses): (Vec<HumanPose>, Vec<HumanPose>) =
            context.unfiltered_poses.iter().copied().partition(|pose| {
                filter_poses_by_overall_confidence(
                    pose,
                    *context.minimum_overall_keypoint_confidence,
                ) && filter_poses_by_visual_referee_confidence(
                    pose,
                    *context.minimum_visual_referee_keypoint_confidence,
                )
            });

        Ok(MainOutputs {
            accepted_human_poses: accepted_human_poses.into(),
            rejected_human_poses: rejected_human_poses.into(),
        })
    }
}

fn filter_poses_by_overall_confidence(
    pose: &HumanPose,
    minimum_overall_keypoint_confidence: f32,
) -> bool {
    let visual_referee_keypoint_indices = OVERALL_KEYPOINT_INDEX_MASK;
    visual_referee_keypoint_indices
        .into_iter()
        .all(|index| pose.keypoints[index].confidence > minimum_overall_keypoint_confidence)
}

fn filter_poses_by_visual_referee_confidence(
    pose: &HumanPose,
    minimum_visual_referee_keypoint_confidence: f32,
) -> bool {
    let visual_referee_keypoint_indices = VISUAL_REFEREE_KEYPOINT_INDEX_MASK;
    visual_referee_keypoint_indices
        .into_iter()
        .all(|index| pose.keypoints[index].confidence > minimum_visual_referee_keypoint_confidence)
}
