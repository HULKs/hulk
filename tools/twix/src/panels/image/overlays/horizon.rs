use color_eyre::Result;
use coordinate_systems::Pixel;
use eframe::epaint::{Color32, Stroke};
use linear_algebra::point;

use crate::{
    panels::image::overlay::Overlay, twix_painter::TwixPainter, value_buffer::BufferHandle,
};

pub struct Horizon {
    horizon: BufferHandle<Option<projection::horizon::Horizon>>,
}

impl Overlay for Horizon {
    const NAME: &'static str = "Horizon";

    fn new(nao: std::sync::Arc<crate::nao::Nao>) -> Self {
        Self {
            horizon: nao.subscribe_value("Control.main_outputs.camera_matrix.horizon"),
        }
    }

    fn paint(&self, painter: &TwixPainter<Pixel>) -> Result<()> {
        let Some(horizon) = self.horizon.get_last_value()?.flatten() else {
            return Ok(());
        };

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
