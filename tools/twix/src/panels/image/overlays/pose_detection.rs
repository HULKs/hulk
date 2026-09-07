use color_eyre::Report;
use eframe::egui::{Align2, Color32};
use ros_z::time::Time;
use types::{
    object_detection::YOLOObjectLabel,
    pose_detection::{Keypoint, Pose},
    time_wrapper::TimeWrapper,
};

use crate::repaint::ObservationContext;

use super::super::image_overlay::{
    ConfidenceThresholdDefinition, ConfidenceThresholdKind, ConfidenceThresholds, ImageOverlay,
    ImageOverlayPainter, OverlayObservation,
};
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
        ConfidenceThresholdKind::BoundingBox,
        "Bounding box confidence",
        "bounding_box_confidence_threshold",
    ),
    ConfidenceThresholdDefinition::new(
        ConfidenceThresholdKind::Keypoint,
        "Keypoint confidence",
        "keypoint_confidence_threshold",
    ),
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
        confidence_thresholds: &ConfidenceThresholds,
    ) {
        let Some(poses) = self.poses.at_time(image_time) else {
            return;
        };
        paint_poses(
            painter,
            &poses.value.inner,
            confidence_thresholds.bounding_box,
            confidence_thresholds.keypoint,
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
        if pose.object.bounding_box.confidence < bounding_box_confidence_threshold {
            continue;
        }
        let color = prediction_colors::PERSON_POSE;
        for (start, end) in POSE_SKELETON_KEYPOINT_LINE_MAPPING {
            if keypoints[start].confidence < keypoint_confidence_threshold
                || keypoints[end].confidence < keypoint_confidence_threshold
            {
                continue;
            }
            painter.detection_line_segment(keypoints[start].point, keypoints[end].point, color);
        }
        for keypoint in keypoints {
            if keypoint.confidence < keypoint_confidence_threshold {
                continue;
            }
            painter.circle_filled(keypoint.point, 1.0, color);
            painter.floating_text(
                keypoint.point,
                Align2::RIGHT_BOTTOM,
                format!("{:.2}", keypoint.confidence),
                Color32::WHITE,
            );
        }
        painter.detection_box(
            pose.object.bounding_box,
            format!("{:.2?}", pose.object.label),
            color,
        );
    }
}
