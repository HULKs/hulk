use std::sync::Arc;

use color_eyre::Result;
use coordinate_systems::Pixel;
use eframe::epaint::Color32;
use types::detected_feet::ClusterPoint;

use crate::{
    nao::Nao,
    panels::image::{cycler_selector::VisionCycler, overlay::Overlay},
    twix_painter::TwixPainter,
    value_buffer::BufferHandle,
};

pub struct FeetDetection {
    cluster_points: BufferHandle<Option<Vec<ClusterPoint>>>,
}

impl Overlay for FeetDetection {
    const NAME: &'static str = "Feet Detection";

    fn new(nao: Arc<Nao>, selected_cycler: VisionCycler) -> Self {
        let cycler_path = selected_cycler.as_path();
        Self {
            cluster_points: nao.subscribe_value(format!(
                "{cycler_path}.additional_outputs.feet_detection.cluster_points"
            )),
        }
    }

    fn paint(&self, painter: &TwixPainter<Pixel>) -> Result<()> {
        let Some(cluster_points) = self.cluster_points.get_last_value()?.flatten() else {
            return Ok(());
        };
        for point in cluster_points {
            painter.circle_filled(point.pixel_coordinates.map(|x| x as f32), 3.0, Color32::RED)
        }
        Ok(())
    }
}
