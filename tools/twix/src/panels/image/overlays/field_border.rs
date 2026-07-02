use color_eyre::Report;
use coordinate_systems::Pixel;
use eframe::egui::{Color32, Stroke};
use linear_algebra::Point2;
use types::{field_border::FieldBorder as FieldBorderData, time_wrapper::TimeWrapper};

use crate::repaint::ObservationContext;

use super::super::image_overlay::{ImageOverlay, ImageOverlayPainter, OverlayObservation};

pub(in crate::panels::image) struct FieldBorderOverlay {
    border_lines: OverlayObservation<TimeWrapper<Option<FieldBorderData>>>,
    candidates: OverlayObservation<Vec<Point2<Pixel>>>,
}

impl ImageOverlay for FieldBorderOverlay {
    const NAME: &'static str = "Field Border";
    const STORAGE_KEY: &'static str = "field_border";

    fn new<C>(context: &C) -> Result<Self, Report>
    where
        C: ObservationContext,
    {
        Ok(Self {
            border_lines: OverlayObservation::new(context, "field_border")?,
            candidates: OverlayObservation::new(context, "field_border_points")?,
        })
    }

    fn paint(&self, painter: &ImageOverlayPainter) {
        let Some(candidates) = self.candidates.latest() else {
            return;
        };
        for point in &candidates.value {
            painter.circle_filled(*point, 2.0, Color32::BLUE);
        }

        let Some(border_lines) = self.border_lines.latest() else {
            return;
        };
        let Some(field_border) = &border_lines.value.inner else {
            return;
        };
        for line in &field_border.border_lines {
            painter.line_segment(
                line.0,
                line.1,
                Stroke::new(3.0, Color32::from_rgb(255, 0, 240)),
            );
        }
    }
}
