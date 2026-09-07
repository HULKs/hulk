use color_eyre::Report;
use ros_z::time::Time;
use types::{pose_detection::RobotPoseDetection, time_wrapper::TimeWrapper};

use crate::repaint::ObservationContext;

use super::{
    super::image_overlay::{
        ConfidenceThresholdDefinition, ImageOverlay, ImageOverlayPainter, OverlayObservation,
    },
    pose::{PoseConfidenceThresholds, PoseStyle, paint_pose},
    prediction_colors,
};

const ROBOT_SKELETON_KEYPOINT_LINE_MAPPING: [(usize, usize); 14] = [
    (0, 1),
    (1, 2),
    (2, 3),
    (3, 4),
    (1, 5),
    (5, 6),
    (6, 7),
    (2, 8),
    (5, 11),
    (8, 11),
    (8, 9),
    (9, 10),
    (11, 12),
    (12, 13),
];
const ROBOT_POSE_CONFIDENCE_THRESHOLDS: [ConfidenceThresholdDefinition; 2] = [
    ConfidenceThresholdDefinition::new(
        "Bounding box confidence",
        "bounding_box_confidence_threshold",
    ),
    ConfidenceThresholdDefinition::new("Keypoint confidence", "keypoint_confidence_threshold"),
];

pub(in crate::panels::image) struct RobotPoseDetectionOverlay {
    poses: OverlayObservation<TimeWrapper<Vec<RobotPoseDetection>>>,
}

impl ImageOverlay for RobotPoseDetectionOverlay {
    const NAME: &'static str = "Robot Pose Detection";
    const STORAGE_KEY: &'static str = "robot_pose_detection";
    const CONFIDENCE_THRESHOLDS: &'static [ConfidenceThresholdDefinition] =
        &ROBOT_POSE_CONFIDENCE_THRESHOLDS;

    fn new<C>(context: &C) -> Result<Self, Report>
    where
        C: ObservationContext,
    {
        Ok(Self {
            poses: OverlayObservation::new(context, "detected_robot_poses")?,
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

        for pose in &poses.value.inner {
            let keypoints = pose.keypoints.as_array();
            paint_pose(
                painter,
                pose.object.bounding_box,
                pose.object.label.into(),
                &keypoints,
                &ROBOT_SKELETON_KEYPOINT_LINE_MAPPING,
                PoseConfidenceThresholds {
                    bounding_box: confidence_thresholds[0],
                    keypoint: confidence_thresholds[1],
                },
                PoseStyle {
                    skeleton: prediction_colors::ROBOT_POSE,
                    keypoint: prediction_colors::ROBOT_POSE,
                    bounding_box: prediction_colors::ROBOT_POSE,
                },
            );
        }
    }

    fn latest_time(&self) -> Option<Time> {
        self.poses.latest_time()
    }
}
