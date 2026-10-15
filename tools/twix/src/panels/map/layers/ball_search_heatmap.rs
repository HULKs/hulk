use color_eyre::{Result, eyre::ensure};
use coordinate_systems::Field;
use eframe::epaint::Color32;
use linear_algebra::point;
use ros_z_debug::{SampleRecord, TopicObservation};
use std::sync::Arc;
use types::{field_dimensions::FieldDimensions, heatmap::Heatmap};

use crate::{backend::RobotBackend, panels::map::layer::Layer};
use twix_visualization::twix_painter::TwixPainter;

pub struct BallSearchHeatmap {
    ball_search_heatmap: TopicObservation<Heatmap>,
}

impl Layer<Field> for BallSearchHeatmap {
    const NAME: &'static str = "Ball Search Heatmap";

    fn new(backend: Arc<RobotBackend>) -> Self {
        let _runtime_handle = backend.runtime_handle().enter();

        let ball_search_heatmap = backend
            .observer()
            .observe_typed("ball_search_heatmap")
            .expect("failed to construct ball search heatmap observer")
            .spawn();

        Self {
            ball_search_heatmap,
        }
    }

    fn paint(
        &self,
        painter: &TwixPainter<Field>,
        field_dimensions: &FieldDimensions,
    ) -> Result<()> {
        let latest_sample = self.ball_search_heatmap.latest();

        let Some(SampleRecord { value: heatmap, .. }) = latest_sample.as_deref() else {
            return Ok(());
        };
        let length = heatmap.length as usize;
        let width = heatmap.width as usize;
        ensure!(length > 0 && width > 0, "heatmap dimensions are empty");
        ensure!(
            length.checked_mul(width) == Some(heatmap.values.len()),
            "heatmap dimensions do not match its number of values"
        );

        let offset = (field_dimensions.length / 2.0, field_dimensions.width / 2.0);
        let cell_length = field_dimensions.length / length as f32;
        let cell_width = field_dimensions.width / width as f32;
        for (x, row) in heatmap.values.chunks_exact(width).enumerate() {
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
