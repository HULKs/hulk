use color_eyre::Result;
use communication::client::{Cycler, CyclerOutput, Output};
use eframe::epaint::{Color32, Stroke};
use types::detected_robots::BoundingBox;

use crate::{
    panels::image::overlay::Overlay, twix_painter::TwixPainter, value_buffer::ValueBuffer,
};

pub struct RobotDetection {
    boxes: ValueBuffer,
}

impl Overlay for RobotDetection {
    const NAME: &'static str = "Robot Detection";

    fn new(nao: std::sync::Arc<crate::nao::Nao>, selected_cycler: Cycler) -> Self {
        Self {
            boxes: nao.subscribe_output(CyclerOutput {
                cycler: selected_cycler,
                output: Output::Main {
                    path: "detected_robots.in_image".to_string(),
                },
            }),
        }
    }

    fn paint(&self, painter: &TwixPainter) -> Result<()> {
        let boxes: Vec<BoundingBox> = self.boxes.require_latest()?;
        for robot_box in &boxes {
            let color = Color32::RED;
            let line_stroke = Stroke::new(2.0, color);
            painter.rect_stroke(
                robot_box.center - robot_box.size / 2.0,
                robot_box.center + robot_box.size / 2.0,
                line_stroke,
            );
        }

        Ok(())
    }
}
