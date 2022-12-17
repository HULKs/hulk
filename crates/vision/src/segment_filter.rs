use color_eyre::Result;
use context_attribute::context;
use framework::MainOutput;
use nalgebra::point;
use types::{FieldBorder, FilteredSegments, ImageSegments, Intensity, ScanGrid, ScanLine, Segment};

pub struct SegmentFilter {}

#[context]
pub struct CreationContext {}

#[context]
pub struct CycleContext {
    pub field_border: Input<Option<FieldBorder>, "field_border?">,
    pub image_segments: Input<ImageSegments, "image_segments">,
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
                vertical_scan_lines: filter_vertical_scan_lines(
                    &context.image_segments.scan_grid.vertical_scan_lines,
                    context.field_border,
                ),
            },
        };
        Ok(MainOutputs {
            filtered_segments: filtered_segments.into(),
        })
    }
}

fn filter_vertical_scan_lines(
    scan_lines: &[ScanLine],
    field_border: Option<&FieldBorder>,
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
    field_border: Option<&FieldBorder>,
) -> Vec<Segment> {
    segments
        .iter()
        .filter(|segment| segment.field_color == Intensity::Low)
        .skip_while(|segment| match field_border {
            Some(field_border) => !field_border
                .is_inside_field(point![scan_line_position as f32, segment.start as f32]),
            None => false,
        })
        .copied()
        .collect()
}
