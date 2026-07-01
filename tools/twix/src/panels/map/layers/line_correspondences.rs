use std::sync::Arc;

use color_eyre::Result;
use eframe::epaint::{Color32, Stroke};

use coordinate_systems::Field;
use geometry::line_segment::LineSegment;
use ros_z_debug::TopicObservation;
use types::field_dimensions::FieldDimensions;

use crate::{backend::RobotBackend, panels::map::layer::Layer, twix_painter::TwixPainter};

pub struct LineCorrespondences {
    correspondence_lines: TopicObservation<Vec<LineSegment<Field>>>,
    measured_lines_in_field: TopicObservation<Vec<LineSegment<Field>>>,
}

impl Layer<Field> for LineCorrespondences {
    const NAME: &'static str = "Line Correspondences";

    fn new(backend: Arc<RobotBackend>) -> Self {
        let _runtime_handle = backend.runtime_handle().enter();

        let correspondence_lines = backend
            .observer()
            .observe_typed("localization/correspondence_lines")
            .expect("failed to construct correspondence lines observer")
            .spawn();

        let measured_lines_in_field = backend
            .observer()
            .observe_typed("localization/measured_lines_in_field")
            .expect("failed to construct measured lines in field observer")
            .spawn();

        Self {
            correspondence_lines,
            measured_lines_in_field,
        }
    }

    fn paint(
        &self,
        painter: &TwixPainter<Field>,
        _field_dimensions: &FieldDimensions,
    ) -> Result<()> {
        if let Some(ros_z_debug::SampleRecord { value: lines, .. }) =
            self.correspondence_lines.latest().as_deref()
        {
            for line in lines {
                painter.line_segment(line.0, line.1, Stroke::new(0.02_f32, Color32::YELLOW));
            }
        }

        if let Some(ros_z_debug::SampleRecord { value: lines, .. }) =
            self.measured_lines_in_field.latest().as_deref()
        {
            for line in lines {
                painter.line_segment(line.0, line.1, Stroke::new(0.04_f32, Color32::RED));
            }
        }
        Ok(())
    }
}
