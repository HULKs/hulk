use std::sync::Arc;

use color_eyre::{eyre::Ok, Result};
use eframe::{
    egui::{Align2, FontId},
    epaint::{Color32, Stroke},
};
use log::warn;

use communication::client::{Cycler, CyclerOutput, Output};
use coordinate_systems::Pixel;
use linear_algebra::point;
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

const DETECTION_IMAGE_WIDTH: f32 = 192.0;
const DETECTION_IMAGE_HEIGHT: f32 = 480.0;
const IMAGE_WIDTH: f32 = 640.0;
const DETECTION_IMAGE_START_X: f32 = (IMAGE_WIDTH - DETECTION_IMAGE_WIDTH) / 2.0;

pub struct PoseDetection {
    accepted_human_poses: ValueBuffer,
    rejected_human_poses: ValueBuffer,
}

impl Overlay for PoseDetection {
    const NAME: &'static str = "Pose Detection";

    fn new(nao: Arc<Nao>, selected_cycler: Cycler) -> Self {
        if selected_cycler != Cycler::VisionTop {
            warn!("PoseDetection only works with the vision top cycler instance!");
        };
        Self {
            accepted_human_poses: nao.subscribe_output(CyclerOutput {
                cycler: Cycler::ObjectDetectionTop,
                output: Output::Main {
                    path: "accepted_human_poses".to_string(),
                },
            }),
            rejected_human_poses: nao.subscribe_output(CyclerOutput {
                cycler: Cycler::ObjectDetectionTop,
                output: Output::Additional {
                    path: "rejected_human_poses".to_string(),
                },
            }),
        }
    }

    fn paint(&self, painter: &crate::twix_painter::TwixPainter<Pixel>) -> Result<()> {
        let rejected_human_poses: Vec<HumanPose> = self.rejected_human_poses.require_latest()?;
        let accepted_human_poses: Vec<HumanPose> = self.accepted_human_poses.require_latest()?;

        paint_poses(
            painter,
            rejected_human_poses,
            Color32::LIGHT_RED,
            Color32::RED,
            Color32::WHITE,
        )?;
        paint_poses(
            painter,
            accepted_human_poses,
            Color32::BLUE,
            Color32::DARK_BLUE,
            Color32::WHITE,
        )?;

        paint_detection_dead_zone(painter);

        Ok(())
    }

    fn config_ui(&mut self, ui: &mut eframe::egui::Ui) {
        ui.horizontal(|ui| {
            ui.add_space(10.0);
        });
    }
}

fn paint_detection_dead_zone(painter: &crate::twix_painter::TwixPainter<Pixel>) {
    painter.rect_filled(
        point![0.0, 0.0],
        point![DETECTION_IMAGE_START_X, DETECTION_IMAGE_HEIGHT],
        Color32::RED.gamma_multiply(0.3),
    );
    painter.rect_filled(
        point![DETECTION_IMAGE_START_X + DETECTION_IMAGE_WIDTH, 0.0],
        point![IMAGE_WIDTH, DETECTION_IMAGE_HEIGHT],
        Color32::RED.gamma_multiply(0.3),
    );
}

fn paint_poses(
    painter: &crate::twix_painter::TwixPainter<Pixel>,
    poses: Vec<HumanPose>,
    line_color: Color32,
    point_color: Color32,
    text_color: Color32,
) -> Result<()> {
    for pose in poses {
        let keypoints: [Keypoint; 17] = pose.keypoints.into();

        // draw skeleton
        for (idx1, idx2) in POSE_SKELETON {
            let keypoint1 = &keypoints[idx1];
            let keypoint2 = &keypoints[idx2];
            painter.line_segment(
                keypoint1.point,
                keypoint2.point,
                Stroke::new(2.0, line_color),
            )
        }

        // draw keypoints
        for (index, keypoint) in keypoints.iter().enumerate() {
            painter.circle_filled(keypoint.point, 2.0, point_color);
            painter.floating_text(
                keypoint.point,
                Align2::RIGHT_BOTTOM,
                format!("{:.2}", keypoint.confidence),
                FontId::default(),
                text_color,
            );
        }

        // draw bounding box
        let bounding_box = pose.bounding_box;
        painter.rect_stroke(
            bounding_box.area.min,
            bounding_box.area.max,
            Stroke::new(2.0, line_color),
        );
        painter.floating_text(
            bounding_box.area.min,
            Align2::RIGHT_BOTTOM,
            format!("{:.2}", bounding_box.confidence),
            FontId::default(),
            text_color,
        );
    }
    Ok(())
}
