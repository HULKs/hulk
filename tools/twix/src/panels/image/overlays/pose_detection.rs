use std::{str::FromStr, sync::Arc};

use color_eyre::Result;
use communication::client::{Cycler, CyclerOutput};
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

pub const KCONF: f32 = 0.1;

pub struct PoseDetection {
    human_poses: Option<ValueBuffer>,
}

impl Overlay for PoseDetection {
    const NAME: &'static str = "Pose Detection";

    fn new(nao: Arc<Nao>, selected_cycler: Cycler) -> Self {
        if selected_cycler != Cycler::VisionTop {
            warn!("PoseDetection only works with the vision top cycler instance!");
            return Self { human_poses: None };
        };
        Self {
            human_poses: Some(
                nao.subscribe_output(
                    CyclerOutput::from_str("DetectionTop.main_outputs.human_poses")
                        .expect("Failed to subscribe to cycler"),
                ),
            ),
        }
    }

    fn paint(&self, painter: &crate::twix_painter::TwixPainter<Pixel>) -> Result<()> {
        let Some(buffer) = self.human_poses.as_ref() else {
            return Ok(());
        };
        let poses: Vec<HumanPose> = buffer.require_latest()?;

        for pose in poses {
            let keypoints: [Keypoint; 17] = pose.keypoints.into();

            // draw keypoints
            for keypoint in keypoints.iter() {
                if keypoint.confidence < KCONF {
                    continue;
                }
                painter.circle_stroke(keypoint.point, 5.0, Stroke::new(2.0, Color32::BLUE));

                painter.text(
                    keypoint.point,
                    Align2::LEFT_TOP,
                    format!("{:.2}", keypoint.confidence),
                    FontId::default(),
                    Color32::WHITE,
                );
            }

            // draw skeleton
            for (idx1, idx2) in POSE_SKELETON {
                let kpt1 = &keypoints[idx1];
                let kpt2 = &keypoints[idx2];
                if kpt1.confidence < KCONF || kpt2.confidence < KCONF {
                    continue;
                };
                painter.line_segment(kpt1.point, kpt2.point, Stroke::new(2.0, Color32::GREEN))
            }
        }
        Ok(())
    }
}
