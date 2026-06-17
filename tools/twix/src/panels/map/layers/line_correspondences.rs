use std::sync::Arc;

use color_eyre::Result;
use eframe::epaint::{Color32, Stroke};

use coordinate_systems::Field;
use geometry::line_segment::LineSegment;
use types::field_dimensions::FieldDimensions;

use crate::{
    backend::TwixBackend,
    panels::map::layer::Layer,
    twix_painter::TwixPainter,
    value_buffer::{BufferHandle, BufferHistory},
};

pub struct LineCorrespondences {
    correspondence_lines: BufferHandle<Vec<LineSegment<Field>>>,
    measured_lines_in_field: BufferHandle<Vec<LineSegment<Field>>>,
}

impl Layer<Field> for LineCorrespondences {
    const NAME: &'static str = "Line Correspondences";

    fn new(backend: Arc<TwixBackend>) -> Self {
        let correspondence_lines = backend.subscribe_buffered_value_with_queue_depth(
            "localization/correspondence_lines",
            BufferHistory::LatestOnly,
            crate::backend::HIGH_RATE_SUBSCRIBER_QUEUE_DEPTH,
        );
        let measured_lines_in_field = backend.subscribe_buffered_value_with_queue_depth(
            "localization/measured_lines_in_field",
            BufferHistory::LatestOnly,
            crate::backend::HIGH_RATE_SUBSCRIBER_QUEUE_DEPTH,
        );
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
        if let Some(lines) = self.correspondence_lines.get_last_value()? {
            for line in lines {
                painter.line_segment(line.0, line.1, Stroke::new(0.02_f32, Color32::YELLOW));
            }
        }

        if let Some(lines) = self.measured_lines_in_field.get_last_value()? {
            for line in lines {
                painter.line_segment(line.0, line.1, Stroke::new(0.04_f32, Color32::RED));
            }
        }
        Ok(())
    }
}
