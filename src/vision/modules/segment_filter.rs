use module_derive::{module, require_some};
use nalgebra::point;

use types::{FieldBorder, FilteredSegments, ImageSegments, Intensity, ScanGrid, ScanLine, Segment};

pub struct SegmentFilter;

#[module(vision)]
#[input(path = field_border, data_type = FieldBorder)]
#[input(path = image_segments, data_type = ImageSegments)]
#[main_output(data_type = FilteredSegments)]
impl SegmentFilter {}

impl SegmentFilter {
    fn new(_context: NewContext) -> anyhow::Result<Self> {
        Ok(Self)
    }

    fn cycle(&mut self, context: CycleContext) -> anyhow::Result<MainOutputs> {
        let image_segments = require_some!(context.image_segments);
        let field_border = require_some!(context.field_border);

        Ok(MainOutputs {
            filtered_segments: Some(FilteredSegments {
                scan_grid: ScanGrid {
                    vertical_scan_lines: filter_vertical_scan_lines(
                        &image_segments.scan_grid.vertical_scan_lines,
                        field_border,
                    ),
                },
            }),
        })
    }
}

fn filter_vertical_scan_lines(
    scan_lines: &[ScanLine],
    field_border: &FieldBorder,
) -> Vec<ScanLine> {
    scan_lines
        .iter()
        .map(|scan_line| ScanLine {
            position: scan_line.position,
            segments: filter_vertical_segments(
                scan_line.position,
                &scan_line.segments,
                field_border,
            ),
        })
        .collect()
}

fn filter_vertical_segments(
    scan_line_position: u16,
    segments: &[Segment],
    field_border: &FieldBorder,
) -> Vec<Segment> {
    segments
        .iter()
        .filter(|segment| segment.field_color == Intensity::Low)
        .skip_while(|segment| {
            !field_border.is_inside_field(point![scan_line_position as f32, segment.start as f32])
        })
        .copied()
        .collect()
}
