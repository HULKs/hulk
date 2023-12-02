use color_eyre::Result;
use communication::client::{Cycler, CyclerOutput, Output};
use eframe::epaint::Color32;
use nalgebra::{DMatrix, Point2};
use std::{str::FromStr, sync::Arc};
use types::field_dimensions::FieldDimensions;

use crate::{
    nao::Nao, panels::map::layer::Layer, twix_painter::TwixPainter, value_buffer::ValueBuffer,
};

pub struct BallSearchHeatmap {
    ball_search_heatmap: ValueBuffer,
}

impl Layer for BallSearchHeatmap {
    const NAME: &'static str = "Ball Search Heatmap";

    fn new(nao: Arc<Nao>) -> Self {
        let ball_search_heatmap = nao.subscribe_output(CyclerOutput {
            cycler: Cycler::Control,
            output: Output::Additional {
                path: "ball_search_heatmap".to_string(),
            },
        });
        Self {
            ball_search_heatmap,
        }
    }

    fn paint(&self, painter: &TwixPainter, field_dimensions: &FieldDimensions) -> Result<()> {
        let heatmap: DMatrix<f32> = self.ball_search_heatmap.parse_latest()?;
        let heatmap_dimensions = (heatmap.ncols(), heatmap.nrows());
        let offset = (field_dimensions.length / 2.0, field_dimensions.width / 2.0);
        let cell_width = field_dimensions.width / heatmap_dimensions.0 as f32;
        let cell_length = field_dimensions.length / heatmap_dimensions.1 as f32;
        for (x, row) in heatmap.row_iter().enumerate() {
            for (y, value) in row.iter().enumerate() {
                painter.rect_filled(
                    Point2::new(
                        x as f32 * cell_length - offset.0,
                        y as f32 * cell_width - offset.1,
                    ),
                    Point2::new(
                        (x + 1) as f32 * cell_length - offset.0,
                        (y + 1) as f32 * cell_width - offset.1,
                    ),
                    Color32::from_rgba_unmultiplied(0, 0, 255, (value * 255.0) as u8),
                );
            }
        }

        Ok(())
    }
}
