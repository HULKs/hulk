use std::sync::Arc;

use color_eyre::Result;
use coordinate_systems::Pixel;
use eframe::egui::{Align2, Color32, FontId, Stroke};
use linear_algebra::point;
use types::pose_detection::{
    HumanPose, Keypoint, OVERALL_KEYPOINT_INDEX_MASK, VISUAL_REFEREE_KEYPOINT_INDEX_MASK,
};

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

const DETECTION_IMAGE_WIDTH: f32 = 192.0;
const DETECTION_IMAGE_HEIGHT: f32 = 480.0;
const IMAGE_WIDTH: f32 = 640.0;
const DETECTION_IMAGE_START_X: f32 = (IMAGE_WIDTH - DETECTION_IMAGE_WIDTH) / 2.0;

pub struct PoseDetection {
    accepted_human_poses: BufferHandle<Vec<HumanPose>>,
    rejected_human_poses: BufferHandle<Vec<HumanPose>>,
}

impl Overlay for PoseDetection {
    const NAME: &'static str = "Pose Detection";

    fn new(nao: Arc<Nao>, selected_cycler: VisionCycler) -> Self {
        let cycler = match selected_cycler {
            VisionCycler::Top => "ObjectDetectionTop",
            VisionCycler::Bottom => "ObjectDetectionBottom",
        };
        let accepted_human_poses =
            nao.subscribe_value(format!("{cycler}.main_outputs.accepted_human_poses"));
        let rejected_human_poses =
            nao.subscribe_value(format!("{cycler}.main_outputs.rejected_human_poses"));
        Self {
            accepted_human_poses,
            rejected_human_poses,
        }
    }

    fn paint(&self, painter: &crate::twix_painter::TwixPainter<Pixel>) -> Result<()> {
        let Some(accepted_human_poses) = self.accepted_human_poses.get_last_value()? else {
            return Ok(());
        };
        let Some(rejected_human_poses) = self.rejected_human_poses.get_last_value()? else {
            return Ok(());
        };

        paint_poses(
            painter,
            rejected_human_poses,
            Color32::LIGHT_RED,
            Color32::DARK_RED,
            Color32::from_rgb(255, 100, 100),
        )?;
        paint_poses(
            painter,
            accepted_human_poses,
            Color32::BLUE,
            Color32::DARK_BLUE,
            Color32::from_rgb(100, 100, 255),
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
    visual_referee_point_color: Color32,
    overall_point_color: Color32,
) -> Result<()> {
    for pose in poses {
        let keypoints: [Keypoint; 17] = pose.keypoints.into();

        // draw skeleton
        for (idx1, idx2) in POSE_SKELETON {
            painter.line_segment(
                keypoints[idx1].point,
                keypoints[idx2].point,
                Stroke::new(2.0, line_color),
            )
        }

        // draw keypoints
        for (index, keypoint) in keypoints.iter().enumerate() {
            if VISUAL_REFEREE_KEYPOINT_INDEX_MASK.contains(&index) {
                painter.circle_filled(keypoint.point, 2.0, visual_referee_point_color);
                painter.floating_text(
                    keypoint.point,
                    Align2::RIGHT_BOTTOM,
                    format!("{:.2}", keypoint.confidence),
                    FontId::default(),
                    Color32::WHITE,
                );
            } else if OVERALL_KEYPOINT_INDEX_MASK.contains(&index) {
                painter.circle_filled(keypoint.point, 2.0, overall_point_color);
                painter.floating_text(
                    keypoint.point,
                    Align2::RIGHT_BOTTOM,
                    format!("{:.2}", keypoint.confidence),
                    FontId::default(),
                    Color32::WHITE.gamma_multiply(0.8),
                );
            } else {
                painter.circle_filled(keypoint.point, 2.0, Color32::GRAY);
                painter.floating_text(
                    keypoint.point,
                    Align2::RIGHT_BOTTOM,
                    format!("{:.2}", keypoint.confidence),
                    FontId::default(),
                    Color32::GRAY.gamma_multiply(0.8),
                );
            }
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
            Color32::WHITE,
        );
    }
    Ok(())
}
