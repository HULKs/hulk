use std::sync::Arc;

use color_eyre::Result;
use eframe::epaint::{Color32, Stroke};

use coordinate_systems::Field;
use geometry::line_segment::LineSegment;
use ros_z_debug::RetentionPolicy;
use types::field_dimensions::FieldDimensions;

use crate::{
    backend::{TwixBackend, retained_subscription::TypedSubscription},
    panels::map::{latest_value, layer::Layer},
    twix_painter::TwixPainter,
};

pub struct LineCorrespondences {
    correspondence_lines: TypedSubscription<Vec<LineSegment<Field>>>,
    measured_lines_in_field: TypedSubscription<Vec<LineSegment<Field>>>,
}

impl Layer<Field> for LineCorrespondences {
    const NAME: &'static str = "Line Correspondences";

    fn new(backend: Arc<TwixBackend>) -> Self {
        let correspondence_lines = backend.subscribe_typed_retained(
            "localization/correspondence_lines",
            RetentionPolicy::LatestOnly,
            crate::backend::HIGH_RATE_SUBSCRIBER_QUEUE_DEPTH,
        );
        let measured_lines_in_field = backend.subscribe_typed_retained(
            "localization/measured_lines_in_field",
            RetentionPolicy::LatestOnly,
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
        if let Some(lines) = latest_value(&self.correspondence_lines) {
            for line in lines {
                painter.line_segment(line.0, line.1, Stroke::new(0.02_f32, Color32::YELLOW));
            }
        }

        if let Some(lines) = latest_value(&self.measured_lines_in_field) {
            for line in lines {
                painter.line_segment(line.0, line.1, Stroke::new(0.04_f32, Color32::RED));
            }
        }
        Ok(())
    }
}
