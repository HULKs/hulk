use std::sync::Arc;

use color_eyre::{eyre::Ok, Result};
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
    filtered_human_poses: ValueBuffer,
    unfiltered_human_poses: ValueBuffer,
}

impl Overlay for PoseDetection {
    const NAME: &'static str = "Pose Detection";

    fn new(nao: Arc<Nao>, selected_cycler: Cycler) -> Self {
        if selected_cycler != Cycler::VisionTop {
            warn!("PoseDetection only works with the vision top cycler instance!");
        };
        Self {
            filtered_human_poses: nao.subscribe_output(CyclerOutput {
                cycler: Cycler::ObjectDetectionTop,
                output: Output::Main {
                    path: "filtered_human_poses".to_string(),
                },
            }),
            unfiltered_human_poses: nao.subscribe_output(CyclerOutput {
                cycler: Cycler::ObjectDetectionTop,
                output: Output::Main {
                    path: "unfiltered_human_poses".to_string(),
                },
            }),
        }
    }

    fn paint(&self, painter: &crate::twix_painter::TwixPainter<Pixel>) -> Result<()> {
        let unfiltered_human_poses: Vec<HumanPose> =
            self.unfiltered_human_poses.require_latest()?;
        let filtered_human_poses: Vec<HumanPose> = self.filtered_human_poses.require_latest()?;

        paint_poses(painter, unfiltered_human_poses, Color32::GRAY)?;
        paint_poses(painter, filtered_human_poses, Color32::BLUE)?;

        Ok(())
    }
}

fn paint_poses(
    painter: &crate::twix_painter::TwixPainter<Pixel>,
    poses: Vec<HumanPose>,
    draw_color: Color32,
) -> Result<()> {
    for pose in poses {
        let keypoints: [Keypoint; 17] = pose.keypoints.into();
        // draw keypoints
        for keypoint in keypoints.iter() {
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
            painter.line_segment(
                keypoint1.point,
                keypoint2.point,
                Stroke::new(2.0, draw_color),
            )
        }

        // draw bounding box
        let bounding_box = pose.bounding_box;
        painter.rect_stroke(
            bounding_box.area.min,
            bounding_box.area.max,
            Stroke::new(2.0, draw_color),
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
