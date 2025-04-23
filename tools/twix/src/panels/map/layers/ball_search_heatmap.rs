use color_eyre::Result;
use coordinate_systems::Field;
use eframe::epaint::Color32;
use linear_algebra::point;
use ndarray::{Array2, Axis};
use std::sync::Arc;
use types::field_dimensions::FieldDimensions;

use crate::{
    nao::Nao, panels::map::layer::Layer, twix_painter::TwixPainter, value_buffer::BufferHandle,
};

pub struct BallSearchHeatmap {
    ball_search_heatmap: BufferHandle<Option<Array2<f32>>>,
}

impl Layer<Field> for BallSearchHeatmap {
    const NAME: &'static str = "Ball Search Heatmap";

    fn new(nao: Arc<Nao>) -> Self {
        let ball_search_heatmap =
            nao.subscribe_value("Control.additional_outputs.ball_search_heatmap");
        Self {
            ball_search_heatmap,
        }
    }

    fn paint(
        &self,
        painter: &TwixPainter<Field>,
        field_dimensions: &FieldDimensions,
    ) -> Result<()> {
        let Some(heatmap) = self.ball_search_heatmap.get_last_value()?.flatten() else {
            return Ok(());
        };
        let heatmap_dimensions = (heatmap.ncols(), heatmap.nrows());
        let offset = (field_dimensions.length / 2.0, field_dimensions.width / 2.0);
        let cell_width = field_dimensions.width / heatmap_dimensions.0 as f32;
        let cell_length = field_dimensions.length / heatmap_dimensions.1 as f32;
        for (x, row) in heatmap.axis_iter(Axis(0)).enumerate() {
            for (y, value) in row.iter().enumerate() {
                let first_point = point![
                    x as f32 * cell_length - offset.0,
                    y as f32 * cell_width - offset.1,
                ];
                let second_point = point![
                    (x + 1) as f32 * cell_length - offset.0,
                    (y + 1) as f32 * cell_width - offset.1,
                ];
                const HEATMAP_OPACITY_SCALE: f32 = 3.0;
                painter.rect_filled(
                    first_point,
                    second_point,
                    Color32::from_rgba_unmultiplied(
                        0,
                        0,
                        255,
                        (value.powf(1.2) * 255.0 * HEATMAP_OPACITY_SCALE) as u8,
                    ),
                );
            }
        }

        Ok(())
    }
}
