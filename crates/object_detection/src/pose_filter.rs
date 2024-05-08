use color_eyre::Result;
use context_attribute::context;
use framework::MainOutput;
use hardware::{NetworkInterface, PathsInterface};
use serde::{Deserialize, Serialize};
use types::pose_detection::{HumanPose, Keypoint};

#[derive(Deserialize, Serialize)]
pub struct PoseFilter {}

#[context]
pub struct CreationContext {
    hardware_interface: HardwareInterface,
}

#[context]
pub struct CycleContext {
    hardware_interface: HardwareInterface,

    unfiltered_poses: Input<Vec<HumanPose>, "unfiltered_human_poses">,

    overall_keypoint_confidence_threshold:
        Parameter<f32, "object_detection.$cycler_instance.overall_keypoint_confidence_threshold">,
    visual_referee_keypoint_confidence_threshold: Parameter<
        f32,
        "object_detection.$cycler_instance.visual_referee_keypoint_confidence_threshold",
    >,
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

    pub fn cycle(&mut self, context: CycleContext<impl NetworkInterface>) -> Result<MainOutputs> {
        let overall_confidences_filtered_poses = filter_poses_by_overall_confidence(
            context.unfiltered_poses.clone(),
            *context.overall_keypoint_confidence_threshold,
        );

        let visual_referee_confidences_filtered_poses = filter_poses_by_visual_referee_confidence(
            overall_confidences_filtered_poses,
            *context.visual_referee_keypoint_confidence_threshold,
        );

        Ok(MainOutputs {
            filtered_human_poses: visual_referee_confidences_filtered_poses.into(),
        })
    }
}

fn filter_poses_by_overall_confidence(
    pose_candidates: Vec<HumanPose>,
    overall_keypoint_confidence_threshold: f32,
) -> Vec<HumanPose> {
    pose_candidates
        .iter()
        .filter(|pose| {
            pose.keypoints
                .iter()
                .all(|keypoint| keypoint.confidence > overall_keypoint_confidence_threshold)
        })
        .copied()
        .collect()
}

fn filter_poses_by_visual_referee_confidence(
    pose_candidates: Vec<HumanPose>,
    visual_referee_keypoint_confidence_threshold: f32,
) -> Vec<HumanPose> {
    pose_candidates
        .iter()
        .filter(|pose| {
            let visual_referee_keypoint_indices = [0, 1, 5, 6, 7, 8, 9, 10, 15, 16];
            let visual_referee_keypoints: Vec<Keypoint> = visual_referee_keypoint_indices
                .iter()
                .map(|&i| pose.keypoints[i])
                .collect();
            visual_referee_keypoints
                .iter()
                .all(|keypoint| keypoint.confidence > visual_referee_keypoint_confidence_threshold)
        })
        .copied()
        .collect()
}
