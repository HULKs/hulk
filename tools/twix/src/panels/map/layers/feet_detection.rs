use std::sync::Arc;

use color_eyre::Result;
use eframe::epaint::Color32;

use coordinate_systems::Ground;
use types::{
    detected_feet::{ClusterPoint, CountedCluster},
    field_dimensions::FieldDimensions,
};

use crate::{
    nao::Nao, panels::map::layer::Layer, twix_painter::TwixPainter, value_buffer::BufferHandle,
};

pub struct FeetDetection {
    cluster: BufferHandle<Option<Vec<CountedCluster>>>,
    cluster_points: BufferHandle<Option<Vec<ClusterPoint>>>,
}

impl Layer<Ground> for FeetDetection {
    const NAME: &'static str = "FeetDetection";

    fn new(nao: Arc<Nao>) -> Self {
        let cluster =
            nao.subscribe_value("Vision.additional_outputs.feet_detection.clusters_in_ground");
        let cluster_points =
            nao.subscribe_value("Vision.additional_outputs.feet_detection.cluster_points");
        Self {
            cluster,
            cluster_points,
        }
    }

    fn paint(
        &self,
        painter: &TwixPainter<Ground>,
        _field_dimensions: &FieldDimensions,
    ) -> Result<()> {
        if let Some(clusters) = self.cluster.get_last_value()?.flatten() {
            for cluster in clusters {
                let radius = 0.1;
                painter.circle_filled(cluster.mean, radius, Color32::YELLOW);
            }
        }

        if let Some(points) = self.cluster_points.get_last_value()?.flatten() {
            for point in points {
                let radius = 0.02;
                painter.circle_filled(point.position_in_ground, radius, Color32::RED);
            }
        }
        Ok(())
    }
}
