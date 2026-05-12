use std::sync::Arc;

use color_eyre::Result;
use coordinate_systems::Pixel;
use eframe::egui::{Align2, Color32, FontId, Stroke};
use linear_algebra::vector;
use types::{
    object_detection::YOLOObjectLabel,
    pose_detection::{Keypoint, Pose},
};

use crate::{panels::image::overlay::Overlay, robot::Robot, value_buffer::BufferHandle};

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

pub struct PoseDetection {
    poses: BufferHandle<Vec<Pose<YOLOObjectLabel>>>,
}

impl Overlay for PoseDetection {
    const NAME: &'static str = "Pose Detection";

    fn new(nao: Arc<Robot>) -> Self {
        let poses = nao.subscribe_value("Hydra.main_outputs.detected_poses".to_string());
        Self { poses }
    }

    fn paint(&self, painter: &crate::twix_painter::TwixPainter<Pixel>) -> Result<()> {
        let Some(poses) = self.poses.get_last_value()? else {
            return Ok(());
        };

        paint_poses(painter, poses)?;

        Ok(())
    }
}

fn paint_poses(
    painter: &crate::twix_painter::TwixPainter<Pixel>,
    poses: Vec<Pose<YOLOObjectLabel>>,
) -> Result<()> {
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
            )
        }

        for keypoint in keypoints.iter() {
            if keypoint.confidence < KEYPOINT_CONFIDENCE_THRESHOLD {
                continue;
            }

            painter.circle_filled(keypoint.point, 1.0, Color32::BLUE);
            painter.floating_text(
                keypoint.point,
                Align2::RIGHT_BOTTOM,
                format!("{:.2}", keypoint.confidence),
                FontId::default(),
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
            bounding_box.area.min
                + vector!(bounding_box.area.max.x() - bounding_box.area.min.x(), 0.0),
            Align2::RIGHT_TOP,
            format!("{:.2}", bounding_box.confidence),
            FontId::default(),
            Color32::WHITE,
        );
        painter.floating_text(
            bounding_box.area.min
                + vector!(0.0, bounding_box.area.max.y() - bounding_box.area.min.y()),
            Align2::LEFT_BOTTOM,
            format!("{:.2?}", pose.object.label),
            FontId::default(),
            Color32::WHITE,
        );
    }
    Ok(())
}
