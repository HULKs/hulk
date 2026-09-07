use std::sync::Arc;

use color_eyre::Report;
use coordinate_systems::Pixel;
use eframe::egui::{Align2, Color32, DragValue, FontId, Stroke, Ui};
use ros_z::time::Time;
use ros_z_debug::{SampleRecord, query::plan::Selection};
use twix_visualization::twix_painter::TwixPainter;
use types::{
    object_detection::YOLOObjectLabel,
    pose_detection::{Keypoint, Pose},
    time_wrapper::TimeWrapper,
};

use super::super::{
    image_overlay::ImageOverlay,
    inputs::{OverlayInput, OverlayInputs},
};

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

use super::prediction_colors;

pub(in crate::panels::image) struct PoseDetectionOverlay {
    bounding_box_confidence_threshold: f32,
    keypoint_confidence_threshold: f32,
    sample: Option<Arc<SampleRecord<Detections>>>,
}

impl Default for PoseDetectionOverlay {
    fn default() -> Self {
        Self {
            bounding_box_confidence_threshold: 0.5,
            keypoint_confidence_threshold: 0.8,
            sample: None,
        }
    }
}

type Detections = TimeWrapper<Vec<Pose<YOLOObjectLabel>>>;

impl ImageOverlay for PoseDetectionOverlay {
    fn inputs(&self) -> Vec<OverlayInput> {
        vec![OverlayInput::new::<Detections>("detected_poses", Selection::Exact).prefer_frame()]
    }

    fn prepare(&mut self, time: Time, inputs: &mut OverlayInputs) -> Result<(), Report> {
        self.sample = None;
        self.sample = inputs
            .decode(0, time)?
            .map(|records| Arc::clone(records.last()));
        Ok(())
    }

    fn ui(&mut self, ui: &mut Ui) {
        ui.horizontal(|ui| {
            ui.label("Bounding box confidence");
            ui.add(
                DragValue::new(&mut self.bounding_box_confidence_threshold)
                    .range(0.0..=1.0)
                    .speed(0.01)
                    .fixed_decimals(2),
            );
        });
        ui.horizontal(|ui| {
            ui.label("Keypoint confidence");
            ui.add(
                DragValue::new(&mut self.keypoint_confidence_threshold)
                    .range(0.0..=1.0)
                    .speed(0.01)
                    .fixed_decimals(2),
            );
        });
    }

    fn reset_settings(&mut self) {
        *self = Self::default();
    }

    fn paint(&self, painter: &TwixPainter<Pixel>) {
        let Some(poses) = &self.sample else {
            return;
        };
        paint_poses(
            painter,
            &poses.value.inner,
            self.bounding_box_confidence_threshold,
            self.keypoint_confidence_threshold,
        );
    }
}

fn paint_poses(
    painter: &TwixPainter<Pixel>,
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
            painter.line_segment(
                keypoints[start].point,
                keypoints[end].point,
                Stroke::new(1.0, color),
            );
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
                FontId::default(),
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
