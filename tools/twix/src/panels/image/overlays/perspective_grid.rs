use std::sync::Arc;

use color_eyre::Result;
use coordinate_systems::Pixel;
use eframe::epaint::{Color32, Stroke};
use linear_algebra::point;
use types::perspective_grid_candidates::Row;

use crate::{
    nao::Nao,
    panels::image::{cycler_selector::VisionCycler, overlay::Overlay},
    twix_painter::TwixPainter,
    value_buffer::BufferHandle,
};

pub struct PerspectiveGrid {
    perspective_grid: BufferHandle<Option<Vec<Row>>>,
}

impl Overlay for PerspectiveGrid {
    const NAME: &'static str = "Perspective Grid";

    fn new(nao: Arc<Nao>, selected_cycler: VisionCycler) -> Self {
        let cycler_path = selected_cycler.as_path();
        Self {
            perspective_grid: nao.subscribe_value(format!(
                "{cycler_path}.additional_outputs.perspective_grid_ball_sizes"
            )),
        }
    }

    fn paint(&self, painter: &TwixPainter<Pixel>) -> Result<()> {
        let Some(perspective_grid) = self.perspective_grid.get_last_value()?.flatten() else {
            return Ok(());
        };

        for row in perspective_grid {
            painter.circle_stroke(
                point![320.0, row.center_y],
                row.circle_radius,
                Stroke::new(2.0, Color32::WHITE),
            );
        }

        Ok(())
    }
}
