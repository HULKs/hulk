use std::sync::Arc;

use color_eyre::Result;
use eframe::epaint::Color32;

use communication::client::{Cycler, CyclerOutput, Output};
use coordinate_systems::{Field, Ground};
use linear_algebra::{Isometry2, Point2};
use types::{detected_feet::ClusterPoint, field_dimensions::FieldDimensions};

use crate::{
    nao::Nao, panels::map::layer::Layer, twix_painter::TwixPainter, value_buffer::ValueBuffer,
};

pub struct FeetDetection {
    ground_to_field: ValueBuffer,
    cluster_bottom: ValueBuffer,
    cluster_top: ValueBuffer,
    segments_bottom: ValueBuffer,
    segments_top: ValueBuffer,
}

impl Layer for FeetDetection {
    const NAME: &'static str = "FeetDetection";

    fn new(nao: Arc<Nao>) -> Self {
        let ground_to_field = nao.subscribe_output(CyclerOutput {
            cycler: Cycler::Control,
            output: Output::Main {
                path: "ground_to_field".to_string(),
            },
        });
        let cluster_bottom = nao.subscribe_output(CyclerOutput {
            cycler: Cycler::VisionBottom,
            output: Output::Additional {
                path: "feet_detection.clusters_in_ground".to_string(),
            },
        });
        let cluster_top = nao.subscribe_output(CyclerOutput {
            cycler: Cycler::VisionTop,
            output: Output::Additional {
                path: "feet_detection.clusters_in_ground".to_string(),
            },
        });
        let segments_bottom = nao.subscribe_output(CyclerOutput {
            cycler: Cycler::VisionBottom,
            output: Output::Additional {
                path: "feet_detection.cluster_points".to_string(),
            },
        });
        let segments_top = nao.subscribe_output(CyclerOutput {
            cycler: Cycler::VisionTop,
            output: Output::Additional {
                path: "feet_detection.cluster_points".to_string(),
            },
        });
        Self {
            ground_to_field,
            cluster_bottom,
            cluster_top,
            segments_bottom,
            segments_top,
        }
    }

    fn paint(
        &self,
        painter: &TwixPainter<Field>,
        _field_dimensions: &FieldDimensions,
    ) -> Result<()> {
        let ground_to_field: Isometry2<Ground, Field> =
            self.ground_to_field.parse_latest().unwrap_or_default();
        let cluster_points: Vec<Point2<Ground>> = [&self.cluster_bottom, &self.cluster_top]
            .iter()
            .filter_map(|buffer| buffer.parse_latest::<Vec<_>>().ok())
            .flatten()
            .collect();
        for point in cluster_points {
            painter.circle_filled(ground_to_field * point, 0.05, Color32::YELLOW);
        }

        let cluster_points: Vec<ClusterPoint> = [&self.segments_bottom, &self.segments_top]
            .iter()
            .filter_map(|buffer| buffer.parse_latest::<Vec<_>>().ok())
            .flatten()
            .collect();
        for point in cluster_points {
            let radius = 0.02;
            painter.circle_filled(
                ground_to_field * point.position_in_ground,
                radius,
                Color32::RED,
            );
        }
        Ok(())
    }
}
