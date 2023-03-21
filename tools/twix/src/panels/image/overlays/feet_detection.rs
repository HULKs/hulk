use std::sync::Arc;

use color_eyre::Result;
use communication::client::{Cycler, CyclerOutput, Output};
use eframe::epaint::Color32;
use types::ScoredClusterPoint;

use crate::{
    nao::Nao, panels::image::overlay::Overlay, twix_painter::TwixPainter, value_buffer::ValueBuffer,
};

pub struct FeetDetection {
    cluster_points: ValueBuffer,
}

impl Overlay for FeetDetection {
    const NAME: &'static str = "Feet Detection";

    fn new(nao: Arc<Nao>, selected_cycler: Cycler) -> Self {
        Self {
            cluster_points: nao.subscribe_output(CyclerOutput {
                cycler: selected_cycler,
                output: Output::Additional {
                    path: "feet_detection.cluster_points_in_pixel".to_string(),
                },
            }),
        }
    }

    fn paint(&self, painter: &TwixPainter) -> Result<()> {
        let cluster_points: Vec<ScoredClusterPoint> = self.cluster_points.require_latest()?;
        for point in cluster_points {
            painter.circle_filled(point.point, 3.0, Color32::RED)
        }
        Ok(())
    }
}
