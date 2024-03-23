use color_eyre::Result;
use communication::client::{Cycler, CyclerOutput, Output};
use coordinate_systems::Pixel;
use eframe::epaint::{Color32, Stroke};
use linear_algebra::point;
use types::perspective_grid_candidates::Row;

use crate::{
    panels::image::overlay::Overlay, twix_painter::TwixPainter, value_buffer::ValueBuffer,
};

pub struct PerspectiveGrid {
    perspective_grid: ValueBuffer,
}

impl Overlay for PerspectiveGrid {
    const NAME: &'static str = "Perspective Grid";

    fn new(nao: std::sync::Arc<crate::nao::Nao>, selected_cycler: Cycler) -> Self {
        Self {
            perspective_grid: nao.subscribe_output(CyclerOutput {
                cycler: selected_cycler,
                output: Output::Additional {
                    path: "rows".to_string(),
                },
            }),
        }
    }

    fn paint(&self, painter: &TwixPainter<Pixel>) -> Result<()> {
        let perspective_grid: Vec<Row> = self.perspective_grid.parse_latest()?;

        for row in perspective_grid {
            let center = row.center_y;
            let radius = row.circle_radius;

            painter.circle_stroke(
                point![320.0, center],
                radius,
                Stroke::new(2.0, Color32::WHITE),
            );
        }

        Ok(())
    }
}
