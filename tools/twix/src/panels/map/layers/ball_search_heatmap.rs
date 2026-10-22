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

#[cfg(test)]
mod tests {
    use super::*;
    use eframe::egui::{self, Rect, pos2};
    use ros_z::context::ContextBuilder;
    use std::time::Duration;

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn shared_heatmap_message_reaches_map_and_preserves_cell_coordinates() {
        let router = ContextBuilder::default()
            .with_mode("router")
            .disable_multicast_scouting()
            .with_connect_endpoints(std::iter::empty::<&str>())
            .with_listen_endpoints(["tcp/127.0.0.1:0"])
            .build()
            .await
            .unwrap();
        let endpoint = router.session().info().locators().await[0].to_string();
        let backend = Arc::new(
            RobotBackend::new(
                tokio::runtime::Handle::current(),
                Some(endpoint),
                "/heatmap_test".into(),
            )
            .await
            .unwrap(),
        );
        let layer = BallSearchHeatmap::new(backend.clone());
        let publisher = backend
            .node()
            .publisher::<Heatmap>("/heatmap_test/ball_search_heatmap")
            .build()
            .await
            .unwrap();
        // The producer stores x as the outer dimension and y as the inner one.
        // Rectangular cells must cover the complete field in x-then-y order.
        let heatmap = Heatmap {
            length: 3,
            width: 2,
            values: vec![0.01, 0.05, 0.1, 0.15, 0.2, 1.0],
        };
        let received = tokio::time::timeout(Duration::from_secs(5), async {
            while layer.ball_search_heatmap.latest().is_none() {
                publisher.publish(&heatmap).await.unwrap();
                tokio::time::sleep(Duration::from_millis(10)).await;
            }
        })
        .await;
        assert!(
            received.is_ok(),
            "heatmap observer: {:?}",
            layer.ball_search_heatmap.status()
        );

        let context = egui::Context::default();
        let output = context.run_ui(egui::RawInput::default(), |ui| {
            let painter =
                TwixPainter::paint_at(ui, Rect::from_min_max(pos2(0.0, 0.0), pos2(100.0, 100.0)));
            layer
                .paint(
                    &painter,
                    &FieldDimensions {
                        length: 2.25,
                        width: 1.0,
                        ..FieldDimensions::SPL_2025
                    },
                )
                .unwrap();
        });
        let cells: Vec<_> = output
            .shapes
            .iter()
            .filter_map(|shape| {
                if let egui::Shape::Path(path) = &shape.shape {
                    Some(path)
                } else {
                    None
                }
            })
            .collect();
        assert_eq!(cells.len(), 6);
        for (index, cell) in cells.iter().enumerate() {
            let x = (index / 2) as f32 * 0.75 - 1.125;
            let y = (index % 2) as f32 * 0.5 - 0.5;
            assert_eq!(
                cell.points,
                [
                    pos2(x, y),
                    pos2(x + 0.75, y),
                    pos2(x + 0.75, y + 0.5),
                    pos2(x, y + 0.5)
                ]
            );
        }
        assert!(
            cells
                .windows(2)
                .all(|pair| pair[0].fill.a() < pair[1].fill.a())
        );
        assert_eq!(cells[5].fill, Color32::BLUE);
        router.shutdown().unwrap();
    }
}
