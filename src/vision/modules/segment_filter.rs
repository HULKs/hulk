use macros::{module, require_some};
use nalgebra::{point, Point2};

use crate::types::{FieldBorder, FilteredSegments, ImageSegments, Intensity, ScanGrid, ScanLine};

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

        let filtered_segments = Some(FilteredSegments {
            scan_grid: ScanGrid {
                horizontal_scan_lines: Self::filter_scan_lines(
                    true,
                    &image_segments.scan_grid.horizontal_scan_lines,
                    field_border,
                ),
                vertical_scan_lines: Self::filter_scan_lines(
                    false,
                    &image_segments.scan_grid.vertical_scan_lines,
                    field_border,
                ),
            },
        });
        Ok(MainOutputs { filtered_segments })
    }

    fn get_position(is_horizontal: bool, line_position: u16, segment_position: u16) -> Point2<f32> {
        if is_horizontal {
            point![segment_position as f32, line_position as f32]
        } else {
            point![line_position as f32, segment_position as f32]
        }
    }

    fn filter_segments(
        is_horizontal: bool,
        scan_line: &ScanLine,
        field_border: &FieldBorder,
    ) -> ScanLine {
        let segments = scan_line
            .segments
            .iter()
            .skip_while(|segment| {
                !field_border.is_inside_field(SegmentFilter::get_position(
                    is_horizontal,
                    scan_line.position,
                    segment.start,
                ))
            })
            .take_while(|segment| {
                field_border.is_inside_field(SegmentFilter::get_position(
                    is_horizontal,
                    scan_line.position,
                    segment.end,
                ))
            })
            .filter(|segment| segment.field_color == Intensity::Low)
            .cloned()
            .collect();

        ScanLine {
            position: scan_line.position,
            segments,
        }
    }

    fn filter_scan_lines(
        is_horizontal: bool,
        scan_lines: &[ScanLine],
        field_border: &FieldBorder,
    ) -> Vec<ScanLine> {
        scan_lines
            .iter()
            .map(|scan_line| Self::filter_segments(is_horizontal, scan_line, field_border))
            .collect()
    }
}

#[cfg(test)]
mod test {
    use crate::types::{EdgeType, Line, Segment, YCbCr444};

    use super::*;

    #[test]
    fn correct_segments_kept_horizontal() {
        let field = YCbCr444 {
            y: 128,
            cb: 0,
            cr: 0,
        };
        let non_field = YCbCr444 {
            y: 255,
            cb: 0,
            cr: 0,
        };
        let scan_lines = vec![ScanLine {
            position: 42,
            segments: vec![
                Segment {
                    start: 0,
                    end: 3,
                    start_edge_type: EdgeType::Border,
                    end_edge_type: EdgeType::Rising,
                    color: non_field,
                    field_color: Intensity::Low,
                },
                Segment {
                    start: 3,
                    end: 6,
                    start_edge_type: EdgeType::Rising,
                    end_edge_type: EdgeType::Rising,
                    color: field,
                    field_color: Intensity::High,
                },
                Segment {
                    start: 6,
                    end: 10,
                    start_edge_type: EdgeType::Rising,
                    end_edge_type: EdgeType::Falling,
                    color: non_field,
                    field_color: Intensity::Low,
                },
                Segment {
                    start: 10,
                    end: 17,
                    start_edge_type: EdgeType::Falling,
                    end_edge_type: EdgeType::Border,
                    color: non_field,
                    field_color: Intensity::Low,
                },
            ],
        }];
        let field_border = FieldBorder {
            border_lines: vec![
                Line(point![0.0, 43.0], point![10.0, 33.0]),
                Line(point![10.0, 33.0], point![20.0, 143.0]),
            ],
        };

        assert_eq!(
            SegmentFilter::filter_scan_lines(true, &scan_lines, &field_border),
            vec![ScanLine {
                position: 42,
                segments: vec![Segment {
                    start: 6,
                    end: 10,
                    start_edge_type: EdgeType::Rising,
                    end_edge_type: EdgeType::Falling,
                    color: non_field,
                    field_color: Intensity::Low,
                },]
            }]
        );
    }
}
