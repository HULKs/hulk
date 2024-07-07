use color_eyre::Result;
use serde::{Deserialize, Serialize};

use context_attribute::context;
use framework::MainOutput;
use hardware::PathsInterface;
use types::pose_detection::HumanPose;

#[derive(Deserialize, Serialize)]
pub struct PoseFilter {}

#[context]
pub struct CreationContext {
    hardware_interface: HardwareInterface,
}

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
    pub filtered_human_poses: MainOutput<Vec<HumanPose>>,
}

impl PoseFilter {
    pub fn new(_creation_context: CreationContext<impl PathsInterface>) -> Result<Self> {
        Ok(PoseFilter {})
    }

    pub fn cycle(&mut self, context: CycleContext) -> Result<MainOutputs> {
        let filtered_human_poses: Vec<_> = context
            .unfiltered_poses
            .iter()
            .copied()
            .filter(|pose| {
                filter_poses_by_overall_confidence(
                    pose,
                    *context.minimum_overall_keypoint_confidence,
                ) && filter_poses_by_visual_referee_confidence(
                    pose,
                    *context.minimum_visual_referee_keypoint_confidence,
                )
            })
            .collect();

        Ok(MainOutputs {
            filtered_human_poses: filtered_human_poses.into(),
        })
    }
}

fn filter_poses_by_overall_confidence(
    pose: &HumanPose,
    minimum_overall_keypoint_confidence: f32,
) -> bool {
    pose.keypoints
        .iter()
        .all(|keypoint| keypoint.confidence > minimum_overall_keypoint_confidence)
}

fn filter_poses_by_visual_referee_confidence(
    pose: &HumanPose,
    minimum_visual_referee_keypoint_confidence: f32,
) -> bool {
    let visual_referee_keypoint_indices = [0, 1, 5, 6, 7, 8, 9, 10, 15, 16];
    visual_referee_keypoint_indices
        .into_iter()
        .all(|index| pose.keypoints[index].confidence > minimum_visual_referee_keypoint_confidence)
}
