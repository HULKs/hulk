use std::{str::FromStr, sync::Arc};

use color_eyre::Result;
use communication::client::{Cycler, CyclerOutput};
use eframe::{
    emath::Align2,
    epaint::{Color32, FontId, Stroke},
};
use types::object_detection::BoundingBox;

use crate::{
    nao::Nao, panels::image::overlay::Overlay, twix_painter::TwixPainter, value_buffer::ValueBuffer,
};

pub struct SingleShotDetection {
    detections: ValueBuffer,
}

impl Overlay for SingleShotDetection {
    const NAME: &'static str = "Single Shot Detection";

    fn new(nao: Arc<Nao>, selected_cycler: Cycler) -> Self {
        let cycler = match selected_cycler {
            Cycler::VisionTop => Cycler::DetectionTop,
            Cycler::VisionBottom => Cycler::DetectionBottom,
            _ => panic!("SingleShotDetection only works with vision cyclers"),
        };

        Self {
            detections: nao.subscribe_output(
                CyclerOutput::from_str(format!("{cycler}.main_outputs.detections").as_str())
                    .expect("failed to subscripe cycler"),
            ),
        }
    }

    fn paint(&self, painter: &TwixPainter) -> Result<()> {
        let detections: Vec<BoundingBox> = self.detections.require_latest()?;
        for detection in detections.iter() {
            painter.rect_stroke(
                detection.bounding_box.min,
                detection.bounding_box.max,
                Stroke::new(2.0, Color32::RED),
            );
            painter.text(
                detection.bounding_box.min,
                Align2::LEFT_BOTTOM,
                format!("{:?} - {:.2}", detection.class, detection.score),
                FontId::default(),
                Color32::RED,
            );
        }

        Ok(())
    }
}
