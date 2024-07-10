use std::sync::Arc;

use color_eyre::Result;
use eframe::epaint::Color32;

use coordinate_systems::Ground;
use linear_algebra::Point2;
use types::{detected_feet::ClusterPoint, field_dimensions::FieldDimensions};

use crate::{
    nao::Nao, panels::map::layer::Layer, twix_painter::TwixPainter, value_buffer::BufferHandle,
};

pub struct FeetDetection {
    cluster_bottom: BufferHandle<Option<Vec<Point2<Ground>>>>,
    cluster_top: BufferHandle<Option<Vec<Point2<Ground>>>>,
    cluster_points_bottom: BufferHandle<Option<Vec<ClusterPoint>>>,
    cluster_points_top: BufferHandle<Option<Vec<ClusterPoint>>>,
}

impl Layer<Ground> for FeetDetection {
    const NAME: &'static str = "FeetDetection";

    fn new(nao: Arc<Nao>) -> Self {
        let cluster_bottom = nao
            .subscribe_value("VisionBottom.additional_outputs.feet_detection.clusters_in_ground");
        let cluster_top =
            nao.subscribe_value("VisionTop.additional_outputs.feet_detection.clusters_in_ground");
        let cluster_points_bottom =
            nao.subscribe_value("VisionBottom.additional_outputs.feet_detection.cluster_points");
        let cluster_points_top =
            nao.subscribe_value("VisionTop.additional_outputs.feet_detection.cluster_points");
        Self {
            cluster_bottom,
            cluster_top,
            cluster_points_bottom,
            cluster_points_top,
        }
    }

    fn paint(
        &self,
        painter: &TwixPainter<Ground>,
        _field_dimensions: &FieldDimensions,
    ) -> Result<()> {
        if let Some(cluster) = self.cluster_bottom.get_last_value()?.flatten() {
            for point in cluster {
                let radius = 0.1;
                painter.circle_filled(point, radius, Color32::YELLOW);
            }
        }
        if let Some(cluster) = self.cluster_top.get_last_value()?.flatten() {
            for point in cluster {
                let radius = 0.1;
                painter.circle_filled(point, radius, Color32::YELLOW);
            }
        }

        if let Some(points) = self.cluster_points_bottom.get_last_value()?.flatten() {
            for point in points {
                let radius = 0.02;
                painter.circle_filled(point.position_in_ground, radius, Color32::RED);
            }
        }
        if let Some(points) = self.cluster_points_top.get_last_value()?.flatten() {
            for point in points {
                let radius = 0.02;
                painter.circle_filled(point.position_in_ground, radius, Color32::RED);
            }
        }
        Ok(())
    }
}
