use std::sync::Arc;

use color_eyre::Result;
use coordinate_systems::Field;
use eframe::epaint::Color32;
use linear_algebra::point;
use types::{field_dimensions::FieldDimensions, heatmap::Heatmap};

use crate::{
    backend::TwixBackend,
    panels::map::layer::Layer,
    twix_painter::TwixPainter,
    value_buffer::{BufferHandle, BufferHistory},
};

pub struct BallSearchHeatmap {
    ball_search_heatmap: BufferHandle<Heatmap>,
}

impl Layer<Field> for BallSearchHeatmap {
    const NAME: &'static str = "Ball Search Heatmap";

    fn new(backend: Arc<TwixBackend>) -> Self {
        let ball_search_heatmap = backend.subscribe_buffered_value_with_queue_depth(
            "ball_search_heatmap",
            BufferHistory::LatestOnly,
            crate::backend::HIGH_RATE_SUBSCRIBER_QUEUE_DEPTH,
        );
        Self {
            ball_search_heatmap,
        }
    }

    fn paint(
        &self,
        painter: &TwixPainter<Field>,
        field_dimensions: &FieldDimensions,
    ) -> Result<()> {
        let Some(heatmap) = self.ball_search_heatmap.get_last_value()? else {
            return Ok(());
        };
        if heatmap.length == 0 || heatmap.width == 0 {
            return Ok(());
        }

        let heatmap_length = heatmap.length as usize;
        let heatmap_width = heatmap.width as usize;
        let offset = (field_dimensions.length / 2.0, field_dimensions.width / 2.0);
        let cell_width = field_dimensions.width / heatmap_width as f32;
        let cell_length = field_dimensions.length / heatmap_length as f32;
        for x in 0..heatmap_length {
            for y in 0..heatmap_width {
                let value = heatmap
                    .values
                    .get(x * heatmap_width + y)
                    .copied()
                    .unwrap_or_default();
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
