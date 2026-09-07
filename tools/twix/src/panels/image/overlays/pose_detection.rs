use color_eyre::Report;
use ros_z::time::Time;
use types::{
    object_detection::YOLOObjectLabel,
    pose_detection::{Keypoint, Pose},
    time_wrapper::TimeWrapper,
};

use crate::repaint::ObservationContext;

use super::super::image_overlay::{
    ConfidenceThresholdDefinition, ImageOverlay, ImageOverlayPainter, OverlayObservation,
};
use super::pose::{PoseConfidenceThresholds, PoseStyle, paint_pose};
use super::prediction_colors;

const POSE_SKELETON_KEYPOINT_LINE_MAPPING: [(usize, usize); 16] = [
    (0, 1),
    (0, 2),
    (1, 3),
    (2, 4),
    (5, 6),
    (5, 11),
    (6, 12),
    (11, 12),
    (5, 7),
    (6, 8),
    (7, 9),
    (8, 10),
    (11, 13),
    (12, 14),
    (13, 15),
    (14, 16),
];
const POSE_CONFIDENCE_THRESHOLDS: [ConfidenceThresholdDefinition; 2] = [
    ConfidenceThresholdDefinition::new(
        "Bounding box confidence",
        "bounding_box_confidence_threshold",
    ),
    ConfidenceThresholdDefinition::new("Keypoint confidence", "keypoint_confidence_threshold"),
];

pub(in crate::panels::image) struct PoseDetectionOverlay {
    poses: OverlayObservation<TimeWrapper<Vec<Pose<YOLOObjectLabel>>>>,
}

impl ImageOverlay for PoseDetectionOverlay {
    const NAME: &'static str = "Pose Detection";
    const STORAGE_KEY: &'static str = "pose_detection";
    const CONFIDENCE_THRESHOLDS: &'static [ConfidenceThresholdDefinition] =
        &POSE_CONFIDENCE_THRESHOLDS;

    fn new<C>(context: &C) -> Result<Self, Report>
    where
        C: ObservationContext,
    {
        Ok(Self {
            poses: OverlayObservation::new(context, "detected_poses")?,
        })
    }

    fn paint(
        &self,
        painter: &ImageOverlayPainter,
        image_time: Time,
        confidence_thresholds: &[f32],
    ) {
        let Some(poses) = self.poses.at_time(image_time) else {
            return;
        };
        paint_poses(
            painter,
            &poses.value.inner,
            confidence_thresholds[0],
            confidence_thresholds[1],
        );
    }

    fn latest_time(&self) -> Option<Time> {
        self.poses.latest_time()
    }
}

fn paint_poses(
    painter: &ImageOverlayPainter,
    poses: &[Pose<YOLOObjectLabel>],
    bounding_box_confidence_threshold: f32,
    keypoint_confidence_threshold: f32,
) {
    for pose in poses {
        let keypoints: [Keypoint; 17] = pose.keypoints.into();
        paint_pose(
            painter,
            pose.object.bounding_box,
            format!("{:.2?}", pose.object.label),
            &keypoints,
            &POSE_SKELETON_KEYPOINT_LINE_MAPPING,
            PoseConfidenceThresholds {
                bounding_box: bounding_box_confidence_threshold,
                keypoint: keypoint_confidence_threshold,
            },
            PoseStyle {
                skeleton: prediction_colors::PERSON_POSE,
                keypoint: prediction_colors::PERSON_POSE,
                bounding_box: prediction_colors::PERSON_POSE,
            },
        );
    }
}
