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
    paint_unfiltered_poses: bool,
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
            paint_unfiltered_poses: false,
        }
    }

    fn paint(&self, painter: &crate::twix_painter::TwixPainter<Pixel>) -> Result<()> {
        let unfiltered_human_poses: Vec<HumanPose> =
            self.unfiltered_human_poses.require_latest()?;
        let filtered_human_poses: Vec<HumanPose> = self.filtered_human_poses.require_latest()?;

        if self.paint_unfiltered_poses {
            paint_poses(
                painter,
                unfiltered_human_poses,
                Color32::LIGHT_RED,
                Color32::RED,
                Color32::WHITE,
                Align2::RIGHT_BOTTOM,
            )?;
        } else {
            paint_poses(
                painter,
                filtered_human_poses,
                Color32::BLUE,
                Color32::DARK_BLUE,
                Color32::WHITE,
                Align2::RIGHT_BOTTOM,
            )?;
        }

        Ok(())
    }

    fn config_ui(&mut self, ui: &mut eframe::egui::Ui) {
        ui.horizontal(|ui| {
            ui.add_space(10.0);
            ui.checkbox(&mut self.paint_unfiltered_poses, "Unfiltered Poses");
        });
    }
}

fn paint_poses(
    painter: &crate::twix_painter::TwixPainter<Pixel>,
    poses: Vec<HumanPose>,
    line_color: Color32,
    point_color: Color32,
    text_color: Color32,
    align: Align2,
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
        for keypoint in keypoints.iter() {
            painter.floating_text(
                keypoint.point,
                align,
                format!("{:.2}", keypoint.confidence),
                FontId::default(),
                text_color,
            );
            painter.circle_filled(keypoint.point, 2.0, point_color);
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
            align,
            format!("{:.2}", bounding_box.confidence),
            FontId::default(),
            text_color,
        );
    }
    Ok(())
}
