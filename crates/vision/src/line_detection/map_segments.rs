use std::slice::Iter;

use coordinate_systems::Pixel;
use linear_algebra::{point, Point2};
use types::image_segments::{GenericSegment, ScanLine};

use super::segment_merger::SegmentMerger;

pub fn map_segments<Mapping: PositionMapping>(
    scan_lines: Iter<'_, ScanLine>,
    maximum_merge_gap: u16,
) -> impl Iterator<Item = GenericSegment> + '_ {
    scan_lines.flat_map(move |scan_line| {
        SegmentMerger::new(
            scan_line.segments.iter().map(|segment| GenericSegment {
                start: Mapping::map(scan_line.position, segment.start),
                end: Mapping::map(scan_line.position, segment.end),
                start_edge_type: segment.start_edge_type,
                end_edge_type: segment.end_edge_type,
            }),
            maximum_merge_gap,
        )
    })
}

pub trait PositionMapping {
    fn map(scan_line_position: u16, segment_position: u16) -> Point2<Pixel, u16>;
}

pub struct HorizontalMapping;

impl PositionMapping for HorizontalMapping {
    fn map(scan_line_position: u16, segment_position: u16) -> Point2<Pixel, u16> {
        point![segment_position, scan_line_position]
    }
}

pub struct VerticalMapping;

impl PositionMapping for VerticalMapping {
    fn map(scan_line_position: u16, segment_position: u16) -> Point2<Pixel, u16> {
        point![scan_line_position, segment_position]
    }
}
