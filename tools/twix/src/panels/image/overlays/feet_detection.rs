use std::sync::Arc;

use color_eyre::Result;
use coordinate_systems::Pixel;
use eframe::epaint::Color32;
use projection::{camera_matrix::CameraMatrix, Projection};
use types::detected_feet::{ClusterPoint, DetectedFeet};

use crate::{
    nao::Nao,
    panels::image::{cycler_selector::VisionCycler, overlay::Overlay},
    twix_painter::TwixPainter,
    value_buffer::BufferHandle,
};

pub struct FeetDetection {
    camera_matrix: BufferHandle<Option<CameraMatrix>>,
    cluster_points: BufferHandle<Option<Vec<ClusterPoint>>>,
    detected_feet: BufferHandle<DetectedFeet>,
}

impl Overlay for FeetDetection {
    const NAME: &'static str = "Feet Detection";

    fn new(nao: Arc<Nao>, selected_cycler: VisionCycler) -> Self {
        let cycler_path = selected_cycler.as_path();
        Self {
            camera_matrix: nao.subscribe_value(format!("{cycler_path}.main_outputs.camera_matrix")),
            cluster_points: nao.subscribe_value(format!(
                "{cycler_path}.additional_outputs.feet_detection.cluster_points"
            )),
            detected_feet: nao.subscribe_value(format!("{cycler_path}.main_outputs.detected_feet")),
        }
    }

    fn paint(&self, painter: &TwixPainter<Pixel>) -> Result<()> {
        let Some(cluster_points) = self.cluster_points.get_last_value()?.flatten() else {
            return Ok(());
        };
        for point in cluster_points {
            painter.circle_filled(point.pixel_coordinates.map(|x| x as f32), 3.0, Color32::RED)
        }

        let Some(detected_feet) = self.detected_feet.get_last_value()? else {
            return Ok(());
        };

        let Some(camera_matrix) = self.camera_matrix.get_last_value()?.flatten() else {
            return Ok(());
        };

        for foot in detected_feet.positions.iter() {
            let foot_in_pixel = camera_matrix.ground_to_pixel(foot.map(|x| x as f32))?;
            painter.circle_filled(foot_in_pixel, 12.0, Color32::YELLOW);
        }
        Ok(())
    }
}
