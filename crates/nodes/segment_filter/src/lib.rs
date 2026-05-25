use std::sync::Arc;

use color_eyre::Result;

use linear_algebra::point;
use ros_z::prelude::*;
use types::{
    color::Intensity,
    field_border::FieldBorder,
    filtered_segments::FilteredSegments,
    image_segments::{Direction, ImageSegments, ScanGrid, ScanLine, Segment},
};

pub async fn run(ctx: Arc<Context>) -> Result<()> {
    let node = ctx.create_node("segment_filter").build().await?;
    let field_border_sub = node
        .subscriber::<Option<FieldBorder>>("field_border")?
        .build()
        .await?;
    let image_segments_cache = node
        .create_cache::<ImageSegments>("image_segments", 10)?
        .build()
        .await?;
    let filtered_segments_pub = node
        .publisher::<FilteredSegments>("filtered_segments")?
        .build()
        .await?;

    loop {
        let field_border = field_border_sub.recv_with_metadata().await?;
        let time_stamp = field_border.source_time;

        let Some(image_segments) = image_segments_cache.get_nearest(time_stamp) else {
            continue;
        };

        let filtered_segments = FilteredSegments {
            scan_grid: ScanGrid {
                horizontal_scan_lines: filter_scan_lines(
                    &image_segments.scan_grid.horizontal_scan_lines,
                    field_border.as_ref(),
                    Direction::Horizontal,
                ),
                vertical_scan_lines: filter_scan_lines(
                    &image_segments.scan_grid.vertical_scan_lines,
                    field_border.as_ref(),
                    Direction::Vertical,
                ),
            },
        };

        filtered_segments_pub.publish(&filtered_segments).await?;
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
