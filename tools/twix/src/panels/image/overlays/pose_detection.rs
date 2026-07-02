use color_eyre::Report;
use eframe::egui::{Align2, Color32, Stroke};
use linear_algebra::point;
use types::{
    object_detection::YOLOObjectLabel,
    pose_detection::{Keypoint, Pose},
};

use crate::repaint::ObservationContext;

use super::super::image_overlay::{ImageOverlay, ImageOverlayPainter, OverlayObservation};

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
const KEYPOINT_CONFIDENCE_THRESHOLD: f32 = 0.8;

pub(in crate::panels::image) struct PoseDetectionOverlay {
    poses: OverlayObservation<Vec<Pose<YOLOObjectLabel>>>,
}

impl ImageOverlay for PoseDetectionOverlay {
    const NAME: &'static str = "Pose Detection";
    const STORAGE_KEY: &'static str = "pose_detection";

    fn new<C>(context: &C) -> Result<Self, Report>
    where
        C: ObservationContext,
    {
        Ok(Self {
            poses: OverlayObservation::new(context, "detected_poses")?,
        })
    }

    fn paint(&self, painter: &ImageOverlayPainter) {
        let Some(poses) = self.poses.latest() else {
            return;
        };
        paint_poses(painter, &poses.value);
    }
}

fn paint_poses(painter: &ImageOverlayPainter, poses: &[Pose<YOLOObjectLabel>]) {
    for pose in poses {
        let keypoints: [Keypoint; 17] = pose.keypoints.into();

        for (idx1, idx2) in POSE_SKELETON_KEYPOINT_LINE_MAPPING {
            if keypoints[idx1].confidence < KEYPOINT_CONFIDENCE_THRESHOLD
                || keypoints[idx2].confidence < KEYPOINT_CONFIDENCE_THRESHOLD
            {
                continue;
            }

            painter.line_segment(
                keypoints[idx1].point,
                keypoints[idx2].point,
                Stroke::new(2.0, Color32::LIGHT_BLUE.gamma_multiply(0.4)),
            );
        }

        for keypoint in keypoints {
            if keypoint.confidence < KEYPOINT_CONFIDENCE_THRESHOLD {
                continue;
            }

            painter.circle_filled(keypoint.point, 1.0, Color32::BLUE);
            painter.floating_text(
                keypoint.point,
                Align2::RIGHT_BOTTOM,
                format!("{:.2}", keypoint.confidence),
                Color32::WHITE,
            );
        }

        let bounding_box = pose.object.bounding_box;
        painter.rect_stroke(
            bounding_box.area.min,
            bounding_box.area.max,
            Stroke::new(2.0, Color32::DARK_BLUE.gamma_multiply(0.8)),
        );
        painter.floating_text(
            point![bounding_box.area.max.x(), bounding_box.area.min.y()],
            Align2::RIGHT_TOP,
            format!("{:.2}", bounding_box.confidence),
            Color32::WHITE,
        );
        painter.floating_text(
            point![bounding_box.area.min.x(), bounding_box.area.max.y()],
            Align2::LEFT_BOTTOM,
            format!("{:.2?}", pose.object.label),
            Color32::WHITE,
        );
    }
}
