mod line_detection;

use std::{collections::HashSet, sync::Arc};

use color_eyre::Result;

use coordinate_systems::{Ground, Pixel};
use geometry::{line::Line2, line_segment::LineSegment};
use linear_algebra::{Point2, distance};
use ordered_float::NotNan;
use projection::{Projection, camera_matrix::CameraMatrix};
use rand::SeedableRng;
use rand_chacha::ChaChaRng;
use ransac::{Ransac, RansacResult};
use ros_z::prelude::*;
use types::{
    filtered_segments::FilteredSegments,
    image_segments::GenericSegment,
    line_data::{LineData, LineDiscardReason},
    parameters::LineDetectionParameters,
    time_wrapper::TimeWrapper,
    ycbcr422_image::YCbCr422Image,
};

use crate::line_detection::{
    checks::{has_opposite_gradients, is_in_length_range, is_non_field_segment},
    iter_if::iter_if,
    map_segments::{HorizontalMapping, VerticalMapping, map_segments},
};

pub async fn run(ctx: Arc<Context>) -> Result<()> {
    let node = ctx.create_node("line_detection").build().await?;

    let parameters = node.bind_parameter_as::<LineDetectionParameters>("line_detection")?;
    let camera_matrix_cache = node
        .create_cache::<TimeWrapper<CameraMatrix>>("camera_matrix", 10)?
        .with_stamp(|w: &TimeWrapper<CameraMatrix>| w.time)
        .build()
        .await?;
    let filtered_segments_sub = node
        .subscriber::<TimeWrapper<FilteredSegments>>("filtered_segments")?
        .build()
        .await?;
    let image_cache = node
        .create_cache::<TimeWrapper<YCbCr422Image>>("inputs/ycbcr422_image", 10)?
        .with_stamp(|w: &TimeWrapper<YCbCr422Image>| w.time)
        .build()
        .await?;
    let lines_in_image_pub = node
        .publisher::<Vec<LineSegment<Pixel>>>("line_detection/lines_in_image")?
        .build()
        .await?;
    // TODO: restructure type layout here, do not use blank tuples
    // let discarded_lines_pub = node
    //     .publisher::<Vec<(LineSegment<Pixel>, LineDiscardReason)>>("discarded_lines")
    //     .build()
    //     .await
    //     .into_eyre()?;
    let filtered_segments_output_pub = node
        .publisher::<Vec<GenericSegment>>("line_detection/filtered_segments")?
        .build()
        .await?;
    let line_data_pub = node
        .publisher::<TimeWrapper<Option<LineData>>>("line_data")?
        .build()
        .await?;

    let mut random_state = ChaChaRng::from_os_rng();

    loop {
        let parameters = parameters.snapshot().typed().clone();

        let timed_filtered_segments = filtered_segments_sub.recv().await?;
        let time_stamp = timed_filtered_segments.time;
        let filtered_segments = timed_filtered_segments.inner;

        let (Some(timed_camera_matrix), Some(timed_image)) = (
            camera_matrix_cache.get_nearest(time_stamp),
            image_cache.get_nearest(time_stamp),
        ) else {
            continue;
        };
        let image = timed_image.inner.clone();
        let camera_matrix = timed_camera_matrix.inner.clone();

        let DetectLinesResult(_discarded_lines, used_segments, lines_in_ground, filtered_segments) =
            detect_lines(
                &parameters,
                &filtered_segments,
                &camera_matrix,
                &image,
                &mut random_state,
            );

        let lines_in_image = lines_in_ground
            .iter()
            .map(|line| {
                LineSegment(
                    camera_matrix.ground_to_pixel(line.0).unwrap(),
                    camera_matrix.ground_to_pixel(line.1).unwrap(),
                )
            })
            .collect();

        lines_in_image_pub.publish(&lines_in_image).await?;

        filtered_segments_output_pub
            .publish(&filtered_segments)
            .await?;

        // let discarded_lines = discarded_lines
        //     .into_iter()
        //     .map(|(line, discard_reason)| {
        //         (
        //             LineSegment(
        //                 camera_matrix.ground_to_pixel(line.point).unwrap(),
        //                 camera_matrix
        //                     .ground_to_pixel(line.point + line.direction)
        //                     .unwrap(),
        //             ),
        //             discard_reason,
        //         )
        //     })
        //     .collect();

        // discarded_lines_pub
        //     .publish(discarded_lines)
        //     .await?;

        line_data_pub
            .publish(&TimeWrapper {
                time: time_stamp,
                inner: Some(LineData {
                    lines: lines_in_ground,
                    used_segments,
                }),
            })
            .await?;
    }
}

struct DetectLinesResult(
    Vec<(Line2<Ground>, LineDiscardReason)>,
    HashSet<Point2<Pixel, u16>>,
    Vec<LineSegment<Ground>>,
    Vec<GenericSegment>,
);

fn detect_lines(
    parameters: &LineDetectionParameters,
    filtered_segments: &FilteredSegments,
    camera_matrix: &CameraMatrix,
    image: &YCbCr422Image,
    random_state: &mut ChaChaRng,
) -> DetectLinesResult {
    let mut discarded_lines = Vec::new();

    let horizontal_scan_lines = iter_if(
        parameters.use_horizontal_segments,
        filtered_segments.scan_grid.horizontal_scan_lines.iter(),
    );
    let vertical_scan_lines = iter_if(
        parameters.use_vertical_segments,
        filtered_segments.scan_grid.vertical_scan_lines.iter(),
    );

    let horizontal_segments = map_segments::<HorizontalMapping>(
        horizontal_scan_lines,
        parameters.maximum_merge_gap_in_pixels,
    );
    let vertical_segments = map_segments::<VerticalMapping>(
        vertical_scan_lines,
        parameters.maximum_merge_gap_in_pixels,
    );

    let filtered_segments = horizontal_segments
        .chain(vertical_segments)
        .filter(|segment| !parameters.check_edge_types || is_non_field_segment(segment))
        .filter(|segment| {
            !parameters.check_line_segments_projection
                || is_in_length_range(
                    segment,
                    camera_matrix,
                    &parameters.allowed_projected_segment_length,
                )
        })
        .filter(|segment| {
            !parameters.check_edge_gradient
                || has_opposite_gradients(
                    segment,
                    image,
                    parameters.gradient_alignment,
                    parameters.gradient_sobel_stride,
                )
        })
        .collect::<Vec<_>>();

    let (line_points, used_segments): (Vec<Point2<Ground>>, HashSet<Point2<Pixel, u16>>) =
        filtered_segments
            .iter()
            .filter_map(|segment| {
                Some((
                    camera_matrix
                        .pixel_to_ground(segment.center().cast())
                        .ok()?,
                    segment.start,
                ))
            })
            .unzip();

    let mut ransac = Ransac::new(line_points);
    let mut lines_in_ground = Vec::new();
    for _ in 0..parameters.maximum_number_of_lines {
        if ransac.unused_points.len() < parameters.minimum_number_of_points_on_line {
            break;
        }
        let RansacResult {
            line: ransac_line,
            used_points,
        } = ransac.next_line(
            random_state,
            parameters.ransac_iterations,
            parameters.maximum_fit_distance_in_ground,
            parameters.maximum_fit_distance_in_ground + parameters.margin_for_point_inclusion,
        );
        let ransac_line =
            ransac_line.expect("Insufficient number of line points. Cannot fit line.");
        if used_points.len() < parameters.minimum_number_of_points_on_line {
            discarded_lines.push((ransac_line, LineDiscardReason::TooFewPoints));
            break;
        }
        let mut points_with_projection_onto_line: Vec<_> = used_points
            .iter()
            .map(|&point| (point, ransac_line.closest_point(point)))
            .collect();
        points_with_projection_onto_line.sort_by_key(|(_point, projected_point)| {
            NotNan::new(projected_point.x()).expect("Tried to compare NaN")
        });
        let split_index = (1..points_with_projection_onto_line.len())
            .find(|&index| {
                distance(
                    points_with_projection_onto_line[index - 1].1,
                    points_with_projection_onto_line[index].1,
                ) > parameters.maximum_gap_on_line
            })
            .unwrap_or(points_with_projection_onto_line.len());
        let after_gap = points_with_projection_onto_line.split_off(split_index);
        ransac
            .unused_points
            .extend(after_gap.iter().map(|(point, _projected_point)| point));
        if points_with_projection_onto_line.len() < parameters.minimum_number_of_points_on_line {
            // just drop and ignore this line
            discarded_lines.push((ransac_line, LineDiscardReason::TooFewPoints));
            continue;
        }

        let Some((_, projected_start_point)) = points_with_projection_onto_line.first().copied()
        else {
            break;
        };
        let Some((_, projected_end_point)) = points_with_projection_onto_line.last().copied()
        else {
            break;
        };

        let line_in_ground = LineSegment(projected_start_point, projected_end_point);
        let line_length_ground = line_in_ground.length();
        let is_too_short = parameters.check_line_length
            && line_length_ground < parameters.allowed_line_length_in_field.start;
        let is_too_long = parameters.check_line_length
            && line_length_ground > parameters.allowed_line_length_in_field.end;
        if is_too_short {
            discarded_lines.push((ransac_line, LineDiscardReason::LineTooShort));
            continue;
        }
        if is_too_long {
            discarded_lines.push((ransac_line, LineDiscardReason::LineTooLong));
            continue;
        }

        let is_too_far = parameters.check_line_distance
            && line_in_ground.center().coords().norm() > parameters.maximum_distance_to_robot;
        if is_too_far {
            discarded_lines.push((ransac_line, LineDiscardReason::TooFarAway));
            continue;
        }

        lines_in_ground.push(line_in_ground);
    }
    DetectLinesResult(
        discarded_lines,
        used_segments,
        lines_in_ground,
        filtered_segments,
    )
}
