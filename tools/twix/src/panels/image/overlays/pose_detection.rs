use std::sync::Arc;

use color_eyre::Result;
use communication::client::{Cycler, CyclerOutput, Output};
use coordinate_systems::Pixel;
use eframe::{
    egui::{Align2, FontId},
    epaint::{Color32, Stroke},
};
use log::warn;
use types::pose_detection::{HumanPose, Keypoint};

use crate::{nao::Nao, panels::image::overlay::Overlay, value_buffer::ValueBuffer};

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
    human_poses: Option<ValueBuffer>,
    keypoint_confidence_threshold: Option<ValueBuffer>,
}

impl Overlay for PoseDetection {
    const NAME: &'static str = "Pose Detection";

    fn new(nao: Arc<Nao>, selected_cycler: Cycler) -> Self {
        if selected_cycler != Cycler::VisionTop {
            warn!("PoseDetection only works with the vision top cycler instance!");
            return Self {
                human_poses: None,
                keypoint_confidence_threshold: None,
            };
        };
        Self {
            human_poses: Some(nao.subscribe_output(CyclerOutput {
                cycler: Cycler::ObjectDetectionTop,
                output: Output::Main {
                    path: "human_poses".to_string(),
                },
            })),
            keypoint_confidence_threshold: Some(nao.subscribe_parameter(
                "object_detection.object_detection_top.keypoint_confidence_threshold",
            )),
        }
    }

    fn paint(&self, painter: &crate::twix_painter::TwixPainter<Pixel>) -> Result<()> {
        let (Some(human_poses), Some(keypoint_confidence_threshold)) = (
            self.human_poses.as_ref(),
            self.keypoint_confidence_threshold.as_ref(),
        ) else {
            return Ok(());
        };
        let human_poses: Vec<HumanPose> = human_poses.require_latest()?;
        let keypoint_confidence_threshold: f32 = keypoint_confidence_threshold.require_latest()?;

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

            // draw bounding box
            let bounding_box = pose.bounding_box;
            painter.rect_stroke(
                bounding_box.area.min,
                bounding_box.area.max,
                Stroke::new(2.0, Color32::BLUE),
            );
            painter.floating_text(
                bounding_box.area.min,
                Align2::RIGHT_TOP,
                format!("{:.2}", bounding_box.confidence),
                FontId::default(),
                Color32::WHITE,
            );
        }
        Ok(())
    }
}
