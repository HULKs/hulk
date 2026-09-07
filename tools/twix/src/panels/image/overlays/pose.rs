use eframe::egui::{Align2, Color32};
use types::{bounding_box::BoundingBox, pose_detection::Keypoint};

use super::super::image_overlay::ImageOverlayPainter;

#[derive(Clone, Copy)]
pub(super) struct PoseStyle {
    pub(super) skeleton: Color32,
    pub(super) keypoint: Color32,
    pub(super) bounding_box: Color32,
}

#[derive(Clone, Copy)]
pub(super) struct PoseConfidenceThresholds {
    pub(super) bounding_box: f32,
    pub(super) keypoint: f32,
}

pub(super) fn paint_pose<const NUMBER_OF_KEYPOINTS: usize>(
    painter: &ImageOverlayPainter,
    bounding_box: BoundingBox,
    label: String,
    keypoints: &[Keypoint; NUMBER_OF_KEYPOINTS],
    skeleton: &[(usize, usize)],
    confidence_thresholds: PoseConfidenceThresholds,
    style: PoseStyle,
) {
    if bounding_box.confidence < confidence_thresholds.bounding_box {
        return;
    }

    for &(start, end) in skeleton {
        if keypoints[start].confidence < confidence_thresholds.keypoint
            || keypoints[end].confidence < confidence_thresholds.keypoint
        {
            continue;
        }

        painter.detection_line_segment(
            keypoints[start].point,
            keypoints[end].point,
            style.skeleton,
        );
    }

    for keypoint in keypoints {
        if keypoint.confidence < confidence_thresholds.keypoint {
            continue;
        }

        painter.circle_filled(keypoint.point, 1.0, style.keypoint);
        painter.floating_text(
            keypoint.point,
            Align2::RIGHT_BOTTOM,
            format!("{:.2}", keypoint.confidence),
            Color32::WHITE,
        );
    }

    painter.detection_box(bounding_box, label, style.bounding_box);
}
