mod checks;
mod iter_if;
mod map_segments;
mod segment;
mod segment_merger;

use std::{collections::HashSet, ops::Range};

use checks::{has_opposite_gradients, is_in_length_range, is_non_field_segment};
use color_eyre::Result;
use geometry::line_segment::LineSegment;
use iter_if::iter_if;
use map_segments::{map_segments, HorizontalMapping, VerticalMapping};
use rand::SeedableRng;
use rand_chacha::ChaChaRng;
use serde::{Deserialize, Serialize};

use context_attribute::context;
use coordinate_systems::{Ground, Pixel};
use framework::{AdditionalOutput, MainOutput};
use linear_algebra::{distance, Point2};
use ordered_float::NotNan;
use projection::{camera_matrix::CameraMatrix, Projection};
use ransac::{Ransac, RansacResult};
use types::{
    filtered_segments::FilteredSegments,
    line_data::{LineData, LineDiscardReason},
    ycbcr422_image::YCbCr422Image,
};

#[derive(Deserialize, Serialize)]
pub struct LineDetection {
    random_state: ChaChaRng,
}

#[context]
pub struct CreationContext {}

#[context]
pub struct CycleContext {
    lines_in_image: AdditionalOutput<Vec<LineSegment<Pixel>>, "lines_in_image">,
    discarded_lines:
        AdditionalOutput<Vec<(LineSegment<Pixel>, LineDiscardReason)>, "discarded_lines">,
    ransac_input: AdditionalOutput<Vec<Point2<Pixel>>, "ransac_input">,

    use_horizontal_segments:
        Parameter<bool, "line_detection.$cycler_instance.use_horizontal_segments">,
    use_vertical_segments: Parameter<bool, "line_detection.$cycler_instance.use_vertical_segments">,
    allowed_line_length_in_field:
        Parameter<Range<f32>, "line_detection.$cycler_instance.allowed_line_length_in_field">,
    check_edge_gradient: Parameter<bool, "line_detection.$cycler_instance.check_edge_gradient">,
    check_line_distance: Parameter<bool, "line_detection.$cycler_instance.check_line_distance">,
    check_line_length: Parameter<bool, "line_detection.$cycler_instance.check_line_length">,
    check_line_segments_projection:
        Parameter<bool, "line_detection.$cycler_instance.check_line_segments_projection">,
    gradient_alignment: Parameter<f32, "line_detection.$cycler_instance.gradient_alignment">,
    gradient_sobel_stride: Parameter<u32, "line_detection.$cycler_instance.gradient_sobel_stride">,
    margin_for_point_inclusion:
        Parameter<f32, "line_detection.$cycler_instance.margin_for_point_inclusion">,
    maximum_distance_to_robot:
        Parameter<f32, "line_detection.$cycler_instance.maximum_distance_to_robot">,
    maximum_fit_distance_in_ground:
        Parameter<f32, "line_detection.$cycler_instance.maximum_fit_distance_in_ground">,
    maximum_gap_on_line: Parameter<f32, "line_detection.$cycler_instance.maximum_gap_on_line">,
    maximum_merge_gap_in_pixels:
        Parameter<u16, "line_detection.$cycler_instance.maximum_merge_gap_in_pixels">,
    maximum_number_of_lines:
        Parameter<usize, "line_detection.$cycler_instance.maximum_number_of_lines">,
    allowed_projected_segment_length:
        Parameter<Range<f32>, "line_detection.$cycler_instance.allowed_projected_segment_length">,
    minimum_number_of_points_on_line:
        Parameter<usize, "line_detection.$cycler_instance.minimum_number_of_points_on_line">,
    ransac_iterations: Parameter<usize, "line_detection.$cycler_instance.ransac_iterations">,

    camera_matrix: RequiredInput<Option<CameraMatrix>, "camera_matrix?">,
    filtered_segments: Input<FilteredSegments, "filtered_segments">,
    image: Input<YCbCr422Image, "image">,
}

#[context]
#[derive(Default)]
pub struct MainOutputs {
    pub line_data: MainOutput<Option<LineData>>,
}

impl LineDetection {
    pub fn new(_context: CreationContext) -> Result<Self> {
        Ok(Self {
            random_state: ChaChaRng::from_entropy(),
        })
    }

    pub fn cycle(&mut self, mut context: CycleContext) -> Result<MainOutputs> {
        let mut discarded_lines = Vec::new();

        let horizontal_scan_lines = iter_if(
            *context.use_horizontal_segments,
            context
                .filtered_segments
                .scan_grid
                .horizontal_scan_lines
                .iter(),
        );
        let vertical_scan_lines = iter_if(
            *context.use_vertical_segments,
            context
                .filtered_segments
                .scan_grid
                .vertical_scan_lines
                .iter(),
        );

        let horizontal_segments = map_segments::<HorizontalMapping>(
            horizontal_scan_lines,
            *context.maximum_merge_gap_in_pixels,
        );
        let vertical_segments = map_segments::<VerticalMapping>(
            vertical_scan_lines,
            *context.maximum_merge_gap_in_pixels,
        );

        let filtered_segments = horizontal_segments
            .chain(vertical_segments)
            .filter(is_non_field_segment)
            .filter(|segment| {
                !*context.check_line_segments_projection
                    || is_in_length_range(
                        segment,
                        context.camera_matrix,
                        context.allowed_projected_segment_length,
                    )
            })
            .filter(|segment| {
                !*context.check_edge_gradient
                    || has_opposite_gradients(
                        segment,
                        context.image,
                        *context.gradient_alignment,
                        *context.gradient_sobel_stride,
                    )
            });

        let (line_points, used_segments): (Vec<Point2<Ground>>, HashSet<Point2<Pixel, u16>>) =
            filtered_segments
                .filter_map(|segment| {
                    Some((
                        context
                            .camera_matrix
                            .pixel_to_ground(segment.center().cast())
                            .ok()?,
                        segment.start,
                    ))
                })
                .unzip();

        context.ransac_input.fill_if_subscribed(|| {
            line_points
                .iter()
                .map(|point| context.camera_matrix.ground_to_pixel(*point).unwrap())
                .collect()
        });

        let mut ransac = Ransac::new(line_points);
        let mut lines_in_ground = Vec::new();
        for _ in 0..*context.maximum_number_of_lines {
            if ransac.unused_points.len() < *context.minimum_number_of_points_on_line {
                break;
            }
            let RansacResult {
                line: ransac_line,
                used_points,
            } = ransac.next_line(
                &mut self.random_state,
                *context.ransac_iterations,
                *context.maximum_fit_distance_in_ground,
                *context.maximum_fit_distance_in_ground + *context.margin_for_point_inclusion,
            );
            let ransac_line =
                ransac_line.expect("Insufficient number of line points. Cannot fit line.");
            if used_points.len() < *context.minimum_number_of_points_on_line {
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
                    ) > *context.maximum_gap_on_line
                })
                .unwrap_or(points_with_projection_onto_line.len());
            let after_gap = points_with_projection_onto_line.split_off(split_index);
            ransac
                .unused_points
                .extend(after_gap.iter().map(|(point, _projected_point)| point));
            if points_with_projection_onto_line.len() < *context.minimum_number_of_points_on_line {
                // just drop and ignore this line
                discarded_lines.push((ransac_line, LineDiscardReason::TooFewPoints));
                continue;
            }

            let Some((_, projected_start_point)) =
                points_with_projection_onto_line.first().copied()
            else {
                break;
            };
            let Some((_, projected_end_point)) = points_with_projection_onto_line.last().copied()
            else {
                break;
            };

            let line_in_ground = LineSegment(projected_start_point, projected_end_point);
            let line_length_ground = line_in_ground.length();
            let is_too_short = *context.check_line_length
                && line_length_ground < context.allowed_line_length_in_field.start;
            let is_too_long = *context.check_line_length
                && line_length_ground > context.allowed_line_length_in_field.end;
            if is_too_short {
                discarded_lines.push((ransac_line, LineDiscardReason::LineTooShort));
                continue;
            }
            if is_too_long {
                discarded_lines.push((ransac_line, LineDiscardReason::LineTooLong));
                continue;
            }

            let is_too_far = *context.check_line_distance
                && line_in_ground.center().coords().norm() > *context.maximum_distance_to_robot;
            if is_too_far {
                discarded_lines.push((ransac_line, LineDiscardReason::TooFarAway));
                continue;
            }

            lines_in_ground.push(line_in_ground);
        }

        context.lines_in_image.fill_if_subscribed(|| {
            lines_in_ground
                .iter()
                .map(|line| {
                    LineSegment(
                        context.camera_matrix.ground_to_pixel(line.0).unwrap(),
                        context.camera_matrix.ground_to_pixel(line.1).unwrap(),
                    )
                })
                .collect()
        });
        context.discarded_lines.fill_if_subscribed(|| {
            discarded_lines
                .into_iter()
                .map(|(line, discard_reason)| {
                    (
                        LineSegment(
                            context.camera_matrix.ground_to_pixel(line.0).unwrap(),
                            context.camera_matrix.ground_to_pixel(line.1).unwrap(),
                        ),
                        discard_reason,
                    )
                })
                .collect()
        });

        Ok(MainOutputs {
            line_data: Some(LineData {
                lines: lines_in_ground,
                used_segments,
            })
            .into(),
        })
    }
}
