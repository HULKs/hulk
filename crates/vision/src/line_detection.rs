mod checks;
mod iter_if;
mod map_segments;
mod segment_merger;

use std::{collections::HashSet, ops::Range};

use color_eyre::Result;
use geometry::{line::Line, line_segment::LineSegment, two_lines::TwoLines};
use itertools::Itertools;
use rand::SeedableRng;
use rand_chacha::ChaChaRng;
use serde::{Deserialize, Serialize};

use context_attribute::context;
use coordinate_systems::{Ground, Pixel};
use framework::{AdditionalOutput, MainOutput};
use linear_algebra::{distance, Point2};
use projection::{camera_matrix::CameraMatrix, Projection};
use ransac::{Ransac, RansacFeature, RansacLineSegment};
use types::{
    filtered_segments::FilteredSegments,
    image_segments::GenericSegment,
    line_data::{LineData, LineDiscardReason},
    ycbcr422_image::YCbCr422Image,
};

use checks::{has_opposite_gradients, is_in_length_range, is_non_field_segment};
use iter_if::iter_if;
use map_segments::{map_segments, HorizontalMapping, VerticalMapping};

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
    filtered_segments_output:
        AdditionalOutput<Vec<GenericSegment>, "line_detection.filtered_segments">,
    detected_features: AdditionalOutput<Vec<RansacFeature<Pixel>>, "detected_features">,

    use_horizontal_segments:
        Parameter<bool, "line_detection.$cycler_instance.use_horizontal_segments">,
    use_vertical_segments: Parameter<bool, "line_detection.$cycler_instance.use_vertical_segments">,
    allowed_line_length_in_field:
        Parameter<Range<f32>, "line_detection.$cycler_instance.allowed_line_length_in_field">,
    check_edge_types: Parameter<bool, "line_detection.$cycler_instance.check_edge_types">,
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
    maximum_number_of_features:
        Parameter<usize, "line_detection.$cycler_instance.maximum_number_of_features">,
    allowed_projected_segment_length:
        Parameter<Range<f32>, "line_detection.$cycler_instance.allowed_projected_segment_length">,
    minimum_number_of_points_on_line:
        Parameter<usize, "line_detection.$cycler_instance.minimum_number_of_points_on_line">,
    ransac_iterations: Parameter<usize, "line_detection.$cycler_instance.ransac_iterations">,
    ransac_fit_two_lines: Parameter<bool, "line_detection.$cycler_instance.ransac_fit_two_lines">,

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
            .filter(|segment| !*context.check_edge_types || is_non_field_segment(segment))
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
            })
            .collect::<Vec<_>>();

        context
            .filtered_segments_output
            .fill_if_subscribed(|| filtered_segments.clone());

        let (line_points, used_segments): (Vec<Point2<Ground>>, HashSet<Point2<Pixel, u16>>) =
            filtered_segments
                .into_iter()
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

        let mut ransac = Ransac::new(line_points);
        let mut detected_features = Vec::new();
        let mut line_segments = Vec::new();
        let mut discarded_line_segments = Vec::new();

        for _ in 0..*context.maximum_number_of_features {
            if ransac.unused_points.len() < *context.minimum_number_of_points_on_line {
                break;
            }
            let ransac_result = ransac.next_feature(
                &mut self.random_state,
                *context.ransac_iterations,
                *context.ransac_fit_two_lines,
                *context.maximum_fit_distance_in_ground,
                *context.maximum_fit_distance_in_ground + *context.margin_for_point_inclusion,
            );
            detected_features.push(ransac_result.feature.clone());

            for line_segment in ransac_result {
                let RansacLineSegment {
                    line_segment,
                    sorted_used_points,
                } = line_segment;

                if sorted_used_points.len() < *context.minimum_number_of_points_on_line {
                    discarded_line_segments.push((line_segment, LineDiscardReason::TooFewPoints));
                    break;
                }

                let projected_sorted_points: Vec<_> = sorted_used_points
                    .iter()
                    .map(|&point| line_segment.closest_point(point))
                    .collect();

                if let Some((gap_index, _)) =
                    projected_sorted_points.windows(2).find_position(|window| {
                        distance(window[0], window[1]) > *context.maximum_gap_on_line
                    })
                {
                    ransac
                        .unused_points
                        .extend(sorted_used_points.iter().skip(gap_index + 1).copied());

                    if gap_index + 1 < *context.minimum_number_of_points_on_line {
                        discarded_line_segments
                            .push((line_segment, LineDiscardReason::TooFewPoints));
                        break;
                    }
                }

                let line_segment_length = line_segment.length();
                let is_too_short = *context.check_line_length
                    && line_segment_length < context.allowed_line_length_in_field.start;
                let is_too_long = *context.check_line_length
                    && line_segment_length > context.allowed_line_length_in_field.end;
                if is_too_short {
                    discarded_line_segments.push((line_segment, LineDiscardReason::LineTooShort));
                    break;
                }
                if is_too_long {
                    discarded_line_segments.push((line_segment, LineDiscardReason::LineTooLong));
                    break;
                }

                let is_too_far = *context.check_line_distance
                    && line_segment.center().coords().norm() > *context.maximum_distance_to_robot;
                if is_too_far {
                    discarded_line_segments.push((line_segment, LineDiscardReason::TooFarAway));
                    break;
                }

                line_segments.push(line_segment)
            }
        }

        context.detected_features.fill_if_subscribed(|| {
            detected_features
                .into_iter()
                .map(|feature| match feature {
                    RansacFeature::None => RansacFeature::None,
                    RansacFeature::Line(line) => RansacFeature::Line(Line::from_points(
                        context.camera_matrix.ground_to_pixel(line.point).unwrap(),
                        context
                            .camera_matrix
                            .ground_to_pixel(line.point + line.direction)
                            .unwrap(),
                    )),
                    RansacFeature::TwoLines(two_lines) => {
                        let point = context
                            .camera_matrix
                            .ground_to_pixel(two_lines.point)
                            .unwrap();
                        RansacFeature::TwoLines(TwoLines {
                            point,
                            direction1: context
                                .camera_matrix
                                .ground_to_pixel(two_lines.point + two_lines.direction1.normalize())
                                .unwrap_or(point)
                                - point,
                            direction2: context
                                .camera_matrix
                                .ground_to_pixel(two_lines.point + two_lines.direction2.normalize())
                                .unwrap_or(point)
                                - point,
                        })
                    }
                })
                .collect()
        });
        context.lines_in_image.fill_if_subscribed(|| {
            line_segments
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
            discarded_line_segments
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
                lines: line_segments,
                used_segments,
            })
            .into(),
        })
    }
}
