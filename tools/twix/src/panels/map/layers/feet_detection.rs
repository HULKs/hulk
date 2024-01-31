use std::sync::Arc;

use color_eyre::Result;
use eframe::epaint::Color32;
use nalgebra::{Isometry2, Point2};

use types::{detected_feet::ClusterPoint, field_dimensions::FieldDimensions};

use crate::{
    nao::Nao, panels::map::layer::Layer, twix_painter::TwixPainter, value_buffer::ValueBuffer,
};

pub struct FeetDetection {
    robot_to_field: ValueBuffer,
    cluster_bottom: ValueBuffer,
    cluster_top: ValueBuffer,
    segments_bottom: ValueBuffer,
    segments_top: ValueBuffer,
}

impl Layer for FeetDetection {
    const NAME: &'static str = "FeetDetection";

    fn new(nao: Arc<Nao>) -> Self {
        let robot_to_field = nao.subscribe_output("Control.robot_to_field");
        let cluster_bottom = nao.subscribe_output("VisionBottom.feet_detection.clusters_in_ground");
        let cluster_top = nao.subscribe_output("VisionTop.feet_detection.clusters_in_ground");
        let segments_bottom = nao.subscribe_output("VisionBottom.feet_detection.cluster_points");
        let segments_top = nao.subscribe_output("VisionTop.feet_detection.cluster_points");
        
        Self {
            robot_to_field,
            cluster_bottom,
            cluster_top,
            segments_bottom,
            segments_top,
        }
    }

    fn paint(&self, painter: &TwixPainter, _field_dimensions: &FieldDimensions) -> Result<()> {
        let robot_to_field: Isometry2<f32> = self.robot_to_field.parse_latest().unwrap_or_default();
        let cluster_points: Vec<Point2<f32>> = [&self.cluster_bottom, &self.cluster_top]
            .iter()
            .filter_map(|buffer| buffer.parse_latest::<Vec<_>>().ok())
            .flatten()
            .collect();
        for point in cluster_points {
            painter.circle_filled(robot_to_field * point, 0.05, Color32::YELLOW);
        }

        let cluster_points: Vec<ClusterPoint> = [&self.segments_bottom, &self.segments_top]
            .iter()
            .filter_map(|buffer| buffer.parse_latest::<Vec<_>>().ok())
            .flatten()
            .collect();
        for point in cluster_points {
            let radius = 0.02;
            painter.circle_filled(
                robot_to_field * point.position_in_ground,
                radius,
                Color32::RED,
            );
        }
        Ok(())
    }
}
