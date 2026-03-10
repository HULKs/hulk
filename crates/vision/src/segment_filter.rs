use color_eyre::Result;
use context_attribute::context;
use framework::MainOutput;
use linear_algebra::point;
use serde::{Deserialize, Serialize};
use types::{
    color::Intensity,
    field_border::FieldBorder,
    filtered_segments::FilteredSegments,
    image_segments::{Direction, ImageSegments, ScanGrid, ScanLine, Segment},
};

#[derive(Deserialize, Serialize)]
pub struct SegmentFilter {}

#[context]
pub struct CreationContext {}

#[context]
pub struct CycleContext {
    field_border: Input<Option<FieldBorder>, "field_border?">,
    image_segments: Input<ImageSegments, "image_segments">,
}

#[context]
#[derive(Default)]
pub struct MainOutputs {
    pub filtered_segments: MainOutput<FilteredSegments>,
}

impl SegmentFilter {
    pub fn new(_context: CreationContext) -> Result<Self> {
        Ok(Self {})
    }

    pub fn cycle(&mut self, context: CycleContext) -> Result<MainOutputs> {
        let filtered_segments = FilteredSegments {
            scan_grid: ScanGrid {
                horizontal_scan_lines: filter_scan_lines(
                    &context.image_segments.scan_grid.horizontal_scan_lines,
                    context.field_border,
                    Direction::Horizontal,
                ),
                vertical_scan_lines: filter_scan_lines(
                    &context.image_segments.scan_grid.vertical_scan_lines,
                    context.field_border,
                    Direction::Vertical,
                ),
            },
        };
        Ok(MainOutputs {
            filtered_segments: filtered_segments.into(),
        })
    }
}

fn filter_scan_lines(
    scan_lines: &[ScanLine],
    field_border: Option<&FieldBorder>,
    direction: Direction,
) -> Vec<ScanLine> {
    scan_lines
        .iter()
        .map(|scan_line| ScanLine {
            position: scan_line.position,
            segments: filter_segments(
                scan_line.position,
                &scan_line.segments,
                field_border,
                direction,
            ),
        })
        .collect()
}

fn filter_segments(
    scan_line_position: u16,
    segments: &[Segment],
    field_border: Option<&FieldBorder>,
    direction: Direction,
) -> Vec<Segment> {
    segments
        .iter()
        .filter(|segment| segment.field_color == Intensity::Low)
        .skip_while(|segment| match field_border {
            Some(field_border) => {
                let point = match direction {
                    Direction::Horizontal => {
                        point![segment.start as f32, scan_line_position as f32]
                    }
                    Direction::Vertical => point![scan_line_position as f32, segment.start as f32],
                };
                !field_border.is_inside_field(point)
            }
            None => false,
        })
        .copied()
        .collect()
}
