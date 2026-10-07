use color_eyre::Report;
use coordinate_systems::Pixel;
use eframe::egui::{Color32, Stroke};
use ros_z::time::Time;
use types::{field_border::FieldBorder as FieldBorderData, time_wrapper::TimeWrapper};

use crate::repaint::ObservationContext;
use twix_visualization::twix_painter::TwixPainter;

use super::super::image_overlay::{ImageOverlay, OverlayObservation};

pub(in crate::panels::image) struct FieldBorderOverlay {
    border_lines: OverlayObservation<TimeWrapper<Option<FieldBorderData>>>,
}

impl ImageOverlay for FieldBorderOverlay {
    type Sample = std::sync::Arc<ros_z_debug::SampleRecord<TimeWrapper<Option<FieldBorderData>>>>;
    const NAME: &'static str = "Field Border";
    const STORAGE_KEY: &'static str = "field_border";

    fn new<C>(context: &C) -> Result<Self, Report>
    where
        C: ObservationContext,
    {
        Ok(Self {
            border_lines: OverlayObservation::new(context, "field_border")?,
        })
    }

    fn prepare(&self, image_time: Time) -> Option<Self::Sample> {
        // Candidate debug points have no image timestamp and are intentionally omitted.
        self.border_lines.at_time(image_time)
    }

    fn paint(painter: &TwixPainter<Pixel>, border_lines: &Self::Sample) {
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
