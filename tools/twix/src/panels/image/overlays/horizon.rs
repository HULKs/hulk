use std::str::FromStr;

use color_eyre::Result;
use communication::client::{Cycler, CyclerOutput};
use coordinate_systems::Pixel;
use eframe::epaint::{Color32, Stroke};
use linear_algebra::point;

use crate::{
    panels::image::overlay::Overlay, twix_painter::TwixPainter, value_buffer::ValueBuffer,
};

pub struct Horizon {
    horizon: ValueBuffer,
}

impl Overlay for Horizon {
    const NAME: &'static str = "Horizon";

    fn new(nao: std::sync::Arc<crate::nao::Nao>, selected_cycler: Cycler) -> Self {
        let camera_position = match selected_cycler {
            Cycler::VisionTop => "top",
            Cycler::VisionBottom => "bottom",
            cycler => panic!("Invalid vision cycler: {cycler}"),
        };
        Self {
            horizon: nao.subscribe_output(
                CyclerOutput::from_str(&format!(
                    "Control.main.camera_matrices.{}.horizon",
                    camera_position,
                ))
                .unwrap(),
            ),
        }
    }

    fn paint(&self, painter: &TwixPainter<Pixel>) -> Result<()> {
        let horizon: projection::horizon::Horizon = self.horizon.require_latest()?;

        let left_horizon_height = horizon.y_at_x(0.0);
        let right_horizon_height = horizon.y_at_x(640.0);

        painter.line_segment(
            point![0.0, left_horizon_height],
            point![640.0, right_horizon_height],
            Stroke::new(3.0, Color32::GREEN),
        );

        painter.circle_stroke(
            horizon.vanishing_point,
            5.0,
            Stroke::new(3.0, Color32::GREEN),
        );

        Ok(())
    }
}
