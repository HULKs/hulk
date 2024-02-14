use std::sync::Arc;

use color_eyre::Result;
use eframe::epaint::Color32;
use types::detected_feet::ClusterPoint;

use crate::{
    nao::Nao,
    panels::image::overlay::{Overlay, VisionCycler},
    twix_painter::TwixPainter,
    value_buffer::ValueBuffer,
};

pub struct FeetDetection {
    cluster_points: ValueBuffer,
}

impl Overlay for FeetDetection {
    const NAME: &'static str = "Feet Detection";

    fn new(nao: Arc<Nao>, selected_cycler: VisionCycler) -> Self {
        Self {
            cluster_points: nao.subscribe_output(format!(
                "{}.additional_outputs.feet_detection.cluster_points",
                selected_cycler.to_string()
            )),
        }
    }

    fn paint(&self, painter: &TwixPainter) -> Result<()> {
        let cluster_points: Vec<ClusterPoint> = self.cluster_points.require_latest()?;
        for point in cluster_points {
            painter.circle_filled(point.pixel_coordinates.map(|x| x as f32), 3.0, Color32::RED)
        }
        Ok(())
    }
}
