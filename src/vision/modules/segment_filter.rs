use macros::{module, require_some};
use nalgebra::point;

use crate::types::{
    FieldBorder, FilteredSegments, ImageSegments, Intensity, Limb, Line, ScanGrid, ScanLine,
    Segment,
};

pub struct SegmentFilter;

#[module(vision)]
#[input(path = field_border, data_type = FieldBorder)]
#[input(path = image_segments, data_type = ImageSegments)]
#[input(path = projected_limbs, data_type = Vec<Limb>)]
#[main_output(data_type = FilteredSegments)]
impl SegmentFilter {}

impl SegmentFilter {
    fn new(_context: NewContext) -> anyhow::Result<Self> {
        Ok(Self)
    }

    fn cycle(&mut self, context: CycleContext) -> anyhow::Result<MainOutputs> {
        let image_segments = require_some!(context.image_segments);
        let field_border = require_some!(context.field_border);
        let projected_limbs = require_some!(context.projected_limbs);

        Ok(MainOutputs {
            filtered_segments: Some(FilteredSegments {
                scan_grid: ScanGrid {
                    horizontal_scan_lines: filter_horizontal_scan_lines(
                        &image_segments.scan_grid.horizontal_scan_lines,
                        field_border,
                    ),
                    vertical_scan_lines: filter_vertical_scan_lines(
                        &image_segments.scan_grid.vertical_scan_lines,
                        field_border,
                        projected_limbs,
                    ),
                },
            }),
        })
    }
}

fn filter_horizontal_scan_lines(
    scan_lines: &[ScanLine],
    field_border: &FieldBorder,
) -> Vec<ScanLine> {
    scan_lines
        .iter()
        .map(|scan_line| ScanLine {
            position: scan_line.position,
            segments: filter_horizontal_segments(
                scan_line.position,
                &scan_line.segments,
                field_border,
            ),
        })
        .collect()
}

fn filter_horizontal_segments(
    scan_line_position: u16,
    segments: &[Segment],
    field_border: &FieldBorder,
) -> Vec<Segment> {
    segments
        .iter()
        .filter(|segment| segment.field_color == Intensity::Low)
        .skip_while(|segment| {
            !field_border.is_inside_field(point![segment.start as f32, scan_line_position as f32])
        })
        .take_while(|segment| {
            field_border.is_inside_field(point![segment.end as f32, scan_line_position as f32])
        })
        .cloned()
        .collect()
}

fn filter_vertical_scan_lines(
    scan_lines: &[ScanLine],
    field_border: &FieldBorder,
    projected_limbs: &[Limb],
) -> Vec<ScanLine> {
    scan_lines
        .iter()
        .map(|scan_line| ScanLine {
            position: scan_line.position,
            segments: filter_vertical_segments(
                scan_line.position,
                &scan_line.segments,
                field_border,
                projected_limbs,
            ),
        })
        .collect()
}

fn filter_vertical_segments(
    scan_line_position: u16,
    segments: &[Segment],
    field_border: &FieldBorder,
    projected_limbs: &[Limb],
) -> Vec<Segment> {
    segments
        .iter()
        .filter(|segment| segment.field_color == Intensity::Low)
        .skip_while(|segment| {
            !field_border.is_inside_field(point![scan_line_position as f32, segment.start as f32])
        })
        .take_while(|segment| is_above_limbs(2 * scan_line_position, segment.end, projected_limbs))
        .cloned()
        .collect()
}

fn is_above_limbs(
    scan_line_position: u16,
    segment_end_position: u16,
    projected_limbs: &[Limb],
) -> bool {
    let scan_line_position = scan_line_position as f32;
    projected_limbs.iter().all(|limb| {
        match limb
            .pixel_polygon
            .as_slice()
            .windows(2)
            .find(|points| points[0].x <= scan_line_position && points[1].x >= scan_line_position)
        {
            Some(points) => {
                if points[0].x == points[1].x {
                    return (segment_end_position as f32) < f32::min(points[0].y, points[1].y);
                }

                // since Y is pointing downwards, "is above" is actually !Line::is_above()
                !Line(points[0], points[1])
                    .is_above(point![scan_line_position, segment_end_position as f32])
            }
            None => true,
        }
    })
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
            filter_horizontal_scan_lines(&scan_lines, &field_border),
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
