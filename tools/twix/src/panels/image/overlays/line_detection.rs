use color_eyre::Report;
use coordinate_systems::Pixel;
use eframe::egui::{Color32, Stroke};
use geometry::line_segment::LineSegment;
use types::{
    image_segments::{EdgeType, GenericSegment},
    line_data::{DiscardedLine, LineDiscardReason},
};

use crate::repaint::ObservationContext;

use super::super::image_overlay::{ImageOverlay, ImageOverlayPainter, OverlayObservation};

pub(in crate::panels::image) struct LineDetectionOverlay {
    lines_in_image: OverlayObservation<Vec<LineSegment<Pixel>>>,
    discarded_lines: OverlayObservation<Vec<DiscardedLine>>,
    filtered_segments: OverlayObservation<Vec<GenericSegment>>,
}

impl ImageOverlay for LineDetectionOverlay {
    const NAME: &'static str = "Line Detection";
    const STORAGE_KEY: &'static str = "line_detection";

    fn new<C>(context: &C) -> Result<Self, Report>
    where
        C: ObservationContext,
    {
        Ok(Self {
            lines_in_image: OverlayObservation::new(context, "line_detection/lines_in_image")?,
            discarded_lines: OverlayObservation::new(context, "line_detection/discarded_lines")?,
            filtered_segments: OverlayObservation::new(
                context,
                "line_detection/filtered_segments",
            )?,
        })
    }

    fn paint(&self, painter: &ImageOverlayPainter) {
        let (Some(lines), Some(discarded_lines), Some(filtered_segments)) = (
            self.lines_in_image.latest(),
            self.discarded_lines.latest(),
            self.filtered_segments.latest(),
        ) else {
            return;
        };

        for segment in &filtered_segments.value {
            painter.line_segment(
                segment.start.cast(),
                segment.end.cast(),
                Stroke::new(1.0, Color32::BLACK),
            );
            painter.circle_stroke(segment.center().cast(), 2.0, Stroke::new(1.0, Color32::RED));
            painter.circle_filled(
                segment.start.cast(),
                1.0,
                edge_type_to_color(segment.start_edge_type),
            );
            painter.circle_filled(
                segment.end.cast(),
                1.0,
                edge_type_to_color(segment.end_edge_type),
            );
        }

        for discarded_line in &discarded_lines.value {
            let color = match discarded_line.discard_reason {
                LineDiscardReason::TooFewPoints => Color32::YELLOW,
                LineDiscardReason::LineTooShort => Color32::GRAY,
                LineDiscardReason::LineTooLong => Color32::BROWN,
                LineDiscardReason::TooFarAway => Color32::BLACK,
            };
            painter.line_segment(
                discarded_line.line.0,
                discarded_line.line.1,
                Stroke::new(3.0, color),
            );
        }

        for line in &lines.value {
            painter.line_segment(line.0, line.1, Stroke::new(3.0, Color32::ORANGE));
        }
    }
}

fn edge_type_to_color(edge_type: EdgeType) -> Color32 {
    match edge_type {
        EdgeType::Rising => Color32::RED,
        EdgeType::Falling => Color32::BLUE,
        EdgeType::ImageBorder => Color32::GOLD,
        EdgeType::LimbBorder => Color32::BLACK,
    }
}
