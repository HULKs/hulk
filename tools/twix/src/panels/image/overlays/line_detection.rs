use color_eyre::Result;
use eframe::epaint::{Color32, Stroke};

use coordinate_systems::Pixel;
use geometry::line_segment::LineSegment;
use types::{image_segments::GenericSegment, line_data::LineDiscardReason};

use crate::{
    panels::{
        image::{cycler_selector::VisionCycler, overlay::Overlay},
        image_segments::edge_type_to_color,
    },
    twix_painter::TwixPainter,
    value_buffer::BufferHandle,
};

type DiscardedLines = Vec<(LineSegment<Pixel>, LineDiscardReason)>;

pub struct LineDetection {
    lines_in_image: BufferHandle<Option<Vec<LineSegment<Pixel>>>>,
    discarded_lines: BufferHandle<Option<DiscardedLines>>,
    filtered_segments: BufferHandle<Option<Vec<GenericSegment>>>,
}

impl Overlay for LineDetection {
    const NAME: &'static str = "Line Detection";

    fn new(nao: std::sync::Arc<crate::nao::Nao>, selected_cycler: VisionCycler) -> Self {
        let cycler_path = selected_cycler.as_path();
        Self {
            lines_in_image: nao
                .subscribe_value(format!("{cycler_path}.additional_outputs.lines_in_image")),
            discarded_lines: nao
                .subscribe_value(format!("{cycler_path}.additional_outputs.discarded_lines")),
            filtered_segments: nao.subscribe_value(format!(
                "{cycler_path}.additional_outputs.line_detection.filtered_segments"
            )),
        }
    }

    fn paint(&self, painter: &TwixPainter<Pixel>) -> Result<()> {
        let Some(lines_in_image) = self.lines_in_image.get_last_value()?.flatten() else {
            return Ok(());
        };
        let Some(discarded_lines) = self.discarded_lines.get_last_value()?.flatten() else {
            return Ok(());
        };
        let Some(filtered_segments) = self.filtered_segments.get_last_value()?.flatten() else {
            return Ok(());
        };
        for segment in filtered_segments {
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
        for (line, reason) in discarded_lines {
            let color = match reason {
                LineDiscardReason::TooFewPoints => Color32::YELLOW,
                LineDiscardReason::LineTooShort => Color32::GRAY,
                LineDiscardReason::LineTooLong => Color32::BROWN,
                LineDiscardReason::TooFarAway => Color32::BLACK,
            };
            painter.line_segment(line.0, line.1, Stroke::new(3.0, color));
        }
        for line in lines_in_image {
            painter.line_segment(line.0, line.1, Stroke::new(3.0, Color32::BLUE));
        }
        Ok(())
    }
}
