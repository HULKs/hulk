use std::sync::Arc;

use color_eyre::Result;
use coordinate_systems::Pixel;
use eframe::{
    egui::{Align2, FontId},
    epaint::{Color32, Stroke},
};
use types::pose_detection::{HumanPose, Keypoint};

use crate::{
    nao::Nao,
    panels::image::{cycler_selector::VisionCycler, overlay::Overlay},
    value_buffer::BufferHandle,
};

const POSE_SKELETON: [(usize, usize); 16] = [
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

pub struct PoseDetection {
    human_poses: BufferHandle<Vec<HumanPose>>,
    keypoint_confidence_threshold: BufferHandle<f32>,
}

impl Overlay for PoseDetection {
    const NAME: &'static str = "Pose Detection";

    fn new(nao: Arc<Nao>, selected_cycler: VisionCycler) -> Self {
        let cycler = match selected_cycler {
            VisionCycler::Top => "ObjectDetectionTop",
            VisionCycler::Bottom => "ObjectDetectionBottom",
        };
        let cycler_path = match selected_cycler {
            VisionCycler::Top => "object_detection_top",
            VisionCycler::Bottom => "object_detection_bottom",
        };
        let human_poses = nao.subscribe_value(format!("{cycler}.main_outputs.human_poses"));
        let keypoint_confidence_threshold = nao.subscribe_value(format!(
            "parameters.object_detection.{cycler_path}.keypoint_confidence_threshold"
        ));
        Self {
            human_poses,
            keypoint_confidence_threshold,
        }
    }

    fn paint(&self, painter: &crate::twix_painter::TwixPainter<Pixel>) -> Result<()> {
        let Some(human_poses) = self.human_poses.get_last_value()? else {
            return Ok(());
        };
        let Some(keypoint_confidence_threshold) =
            self.keypoint_confidence_threshold.get_last_value()?
        else {
            return Ok(());
        };

        for pose in human_poses {
            let keypoints: [Keypoint; 17] = pose.keypoints.into();

            // draw keypoints
            for keypoint in keypoints.iter() {
                if keypoint.confidence < keypoint_confidence_threshold {
                    continue;
                }
                painter.circle_stroke(keypoint.point, 5.0, Stroke::new(2.0, Color32::BLUE));

                painter.floating_text(
                    keypoint.point,
                    Align2::LEFT_TOP,
                    format!("{:.2}", keypoint.confidence),
                    FontId::default(),
                    Color32::WHITE,
                );
            }

            // draw skeleton
            for (idx1, idx2) in POSE_SKELETON {
                let keypoint1 = &keypoints[idx1];
                let keypoint2 = &keypoints[idx2];
                if keypoint1.confidence < keypoint_confidence_threshold
                    || keypoint2.confidence < keypoint_confidence_threshold
                {
                    continue;
                };
                painter.line_segment(
                    keypoint1.point,
                    keypoint2.point,
                    Stroke::new(2.0, Color32::GREEN),
                )
            }
        }
        Ok(())
    }
}
