use std::{
    f32::consts::PI,
    time::{Duration, Instant, SystemTime},
};

use color_eyre::{eyre::Ok, Result};
use edge_detection::{get_edges_canny, get_edges_canny_imageproc, EdgeSourceType};
use geometry::{
    line::{self, Line2},
    line_segment::LineSegment,
    rectangle::Rectangle,
    Distance,
};
use imageproc::point;
use itertools::{max, Itertools};
use rand::SeedableRng;
use rand_chacha::ChaChaRng;
use serde::{Deserialize, Serialize};

use calibration::center_circle::{circle_points::CenterCirclePoints, fine_tuner::ellifit};
use context_attribute::context;
use coordinate_systems::{Ground, Pixel};

use framework::{deserialize_not_implemented, AdditionalOutput, MainOutput};
use linear_algebra::{distance, point, vector, IntoFramed, Point2};
use projection::{camera_matrix::CameraMatrix, Projection};
use ransac::{
    circles::circle::{RansacCircleWithTransformation, RansacResultCircleWithTransformation},
    Ransac,
};
use types::{
    calibration::{CalibrationCommand, CalibrationFeatureDetectorOutput},
    camera_position::CameraPosition,
    field_dimensions::FieldDimensions,
    filtered_segments::FilteredSegments,
    ycbcr422_image::YCbCr422Image,
};

use crate::hough::{
    get_center_circle_roi, get_hough_line_with_edges, get_hough_line_with_edges_imgproc,
    HoughParams,
};

#[derive(Deserialize, Serialize)]
pub struct CalibrationMeasurementDetection {
    #[serde(skip, default = "deserialize_not_implemented")]
    last_processed_instance: Instant,
}

#[context]
pub struct CreationContext {}
#[context]
pub struct CycleContext {
    tuning_mode:
        Parameter<bool, "calibration_center_circle_detection.$cycler_instance.tuning_mode">,
    field_dimensions: Parameter<FieldDimensions, "field_dimensions">,

    preprocessing_luma_without_difference:
        Parameter<bool, "calibration_center_circle_detection.skip_rgb_based_difference_image">,
    preprocessing_gaussian_sigma:
        Parameter<f32, "calibration_center_circle_detection.gaussian_sigma">,
    canny_low_threshold: Parameter<f32, "calibration_center_circle_detection.canny_low_threshold">,
    canny_high_threshold:
        Parameter<f32, "calibration_center_circle_detection.canny_high_threshold">,
    preprocessing_get_edges_from_segments:
        Parameter<bool, "calibration_center_circle_detection.get_edges_from_segments">,

    ransac_maximum_number_of_circles:
        Parameter<usize, "calibration_center_circle_detection.maximum_number_of_circles">,
    ransac_iterations: Parameter<usize, "calibration_center_circle_detection.ransac_iterations">,
    ransac_circle_inlier_threshold:
        Parameter<f32, "calibration_center_circle_detection.ransac_circle_inlier_threshold">,
    ransac_circle_minimum_circumference_percentage: Parameter<
        f32,
        "calibration_center_circle_detection.ransac_circle_minimum_circumference_percentage",
    >,
    ransac_sample_size_percentage: Parameter<
        Option<f32>,
        "calibration_center_circle_detection.ransac_sample_size_percentage?",
    >,
    refine_hough_rho_bin_size:
        Parameter<usize, "calibration_center_circle_detection.refine.hough_rho_bin_size">,
    refine_hough_threshold:
        Parameter<usize, "calibration_center_circle_detection.refine.hough_threshold">,
    refine_hough_nms_radius:
        Parameter<usize, "calibration_center_circle_detection.refine.hough_nms_radius">,
    // profiling_active: Parameter<bool, "calibration_center_circle_detection.profiling_active">,
    center_line_point_exclusion_distance_factor: Parameter<
        f32,
        "calibration_center_circle_detection.refine.center_line_point_exclusion_distance_factor",
    >,

    run_next_cycle_after_ms:
        Parameter<u64, "calibration_center_circle_detection.run_next_cycle_after_ms">,
    calibration_command: Input<Option<CalibrationCommand>, "control", "calibration_command?">,

    image: Input<YCbCr422Image, "image">,
    camera_matrix: RequiredInput<Option<CameraMatrix>, "camera_matrix?">,
    camera_position: Parameter<CameraPosition, "image_receiver.$cycler_instance.camera_position">,
    filtered_segments: Input<FilteredSegments, "filtered_segments">,

    detected_edge_points: AdditionalOutput<
        Vec<Point2<Pixel>>,
        "calibration_center_circle_detection.detected_edge_points",
    >,
    timings_for_steps_ms: AdditionalOutput<
        Vec<(String, u128)>,
        "calibration_center_circle_detection.timings_for_steps",
    >,
    circles_points_pixel_scores: AdditionalOutput<
        Vec<f32>,
        "calibration_center_circle_detection.circles_points_pixel_scores",
    >,
    circle_lines: AdditionalOutput<
        Vec<LineSegment<Pixel>>,
        "calibration_center_circle_detection.circle_lines",
    >,
    // circle_line_points: AdditionalOutput<
    //     Vec<Point2<Pixel>>,
    //     "calibration_center_circle_detection.circle_line_points",
    // >,
}

#[context]
#[derive(Default)]
pub struct MainOutputs {
    pub calibration_center_circle:
        MainOutput<CalibrationFeatureDetectorOutput<CenterCirclePoints<Pixel>>>,
}

impl CalibrationMeasurementDetection {
    pub fn new(_context: CreationContext) -> Result<Self> {
        Ok(Self {
            last_processed_instance: Instant::now(),
        })
    }

    pub fn cycle(&mut self, mut context: CycleContext) -> Result<MainOutputs> {
        let capture_command_received = context.calibration_command.map_or(false, |command| {
            command.capture && command.camera == *context.camera_position
        });
        let timeout_complete = self.last_processed_instance.elapsed()
            >= Duration::from_millis(*context.run_next_cycle_after_ms);
        if !(timeout_complete && (capture_command_received || *context.tuning_mode)) {
            return Ok(MainOutputs {
                calibration_center_circle: CalibrationFeatureDetectorOutput {
                    cycle_skipped: true,
                    detected_feature: None,
                }
                .into(),
            });
        }

        let processing_start = Instant::now();
        let y_exclusion_threshold = get_y_exclusion_threshold(&context);
        let filtered_points = if *context.preprocessing_get_edges_from_segments {
            get_edges_from_segments(
                context.filtered_segments,
                context
                    .camera_matrix
                    .horizon
                    .map(|h| h.horizon_y_minimum() as u32),
            )
        } else {
            get_edges_from_canny_edge_detection(&context, y_exclusion_threshold)
        };

        let elapsed_time_after_getting_edges = processing_start.elapsed();
        let filtered_calibration_circles_ground =
            detect_and_filter_circles(&filtered_points, &context, y_exclusion_threshold);

        let elapsed_time_after_all_processing = processing_start.elapsed();

        context.circle_lines.fill_if_subscribed(|| {
            // let maximum_score_distance = 10.0;
            // let maximum_inclusion_distance = 10.0;
            // let mut line_ransac = ransac::Ransac::new(filtered_points.to_vec());
            // let mut rng = ChaChaRng::from_entropy();
            // (0..5)
            //     .flat_map(|_| {
            //         line_ransac
            //             .next_line(&mut rng, 10000, maximum_score_distance, maximum_inclusion_distance)
            //             .line
            //             .map(|line| LineSegment(line.point, line.point + line.direction))
            //     })
            //     .collect()
            // filtered_calibration_circles_ground
            //     .iter()
            //     .map(|(_, line, _)| *line)
            //     .collect()

            filtered_calibration_circles_ground
                .iter()
                .flat_map(|(_, lines, _)| lines.clone())
                .collect()
        });
        context
            .detected_edge_points
            .fill_if_subscribed(|| filtered_points);

        context.circles_points_pixel_scores.fill_if_subscribed(|| {
            filtered_calibration_circles_ground
                .iter()
                .map(|(_, _, score)| *score)
                .collect_vec()
        });

        context.timings_for_steps_ms.fill_if_subscribed(|| {
            vec![
                (
                    "edge_detection_ms".to_string(),
                    (elapsed_time_after_getting_edges).as_micros(),
                ),
                (
                    "circle_us".to_string(),
                    (elapsed_time_after_all_processing - elapsed_time_after_getting_edges)
                        .as_micros(),
                ),
                (
                    "elapsed_time_after_all_processing_ms".to_string(),
                    (elapsed_time_after_all_processing).as_millis(),
                ),
            ]
        });

        self.last_processed_instance = Instant::now();

        Ok(MainOutputs {
            calibration_center_circle: CalibrationFeatureDetectorOutput {
                detected_feature: filtered_calibration_circles_ground
                    .first()
                    .map(|(feature, _, _)| feature.clone()),
                cycle_skipped: false,
            }
            .into(),
        })
    }
}

fn refine_center_circle(
    center_circle: &RansacResultCircleWithTransformation<Pixel, Ground>,
    circle_center: Point2<Pixel>,
    ransac_source_points: &[Point2<Pixel>],
    ransac_iters: usize,
    context: &CycleContext,
) -> Option<(
    CenterCirclePoints<Pixel>,
    LineSegment<Pixel>,
    Vec<LineSegment<Pixel>>,
)> {
    if center_circle.used_points_original.len() < 5 {
        return None;
    }
    let circle_points_pixel = &center_circle.used_points_original;
    let roi_padding = 10.0;
    let roi = get_center_circle_roi(circle_points_pixel, (roi_padding, roi_padding));
    let roi_x_range = roi.min.x()..=roi.max.x();
    let roi_y_range = roi.min.y()..=roi.max.y();
    let roi_height = roi.max.y() - roi.min.y();
    let roi_width = roi.max.x() - roi.min.x();

    let roi_points: Vec<_> = ransac_source_points
        .iter()
        .filter(|point| roi_x_range.contains(&point.x()) && roi_y_range.contains(&point.y()))
        .cloned()
        .collect();

    // TODO parametrize?
    let maximum_inclusion_distance = 10.0;

    let min_dim = roi_height.min(roi_width);
    let min_distance_from_center_squared = ((min_dim - roi_padding) * 0.20).powi(2);

    let middle_and_source_lines = get_center_circle_line(
        circle_center,
        context,
        roi,
        &roi_points,
        min_distance_from_center_squared,
    );

    let min_distance_from_line = 6.0f32
        .max(min_dim * *context.center_line_point_exclusion_distance_factor)
        .max(maximum_inclusion_distance);
    // let min_distance_from_center = (min_dim - 2.0 * 5.0) * 0.20;
    // TODO Might be better to filter and combine the already split points by ransac
    middle_and_source_lines.map(|(line, source_lines)| {
        let cleaned_center = line.closest_point(circle_center);
        // let cleaned_center = circle_center;
        let filtered_roi_points: Vec<_> = roi_points
            .iter()
            .filter(|&&point| {
                source_lines
                    .iter()
                    .all(|source_line| source_line.distance_to(point) > min_distance_from_line)
                // line.distance_to(point) > min_distance_from_line
                // && (point - cleaned_center).norm_squared() > min_distance_from_center_squared
            })
            .copied()
            .collect();

        // let arc_clusters = get_arc_clusters(
        //     circle_center,
        //     &filtered_roi_points,
        //     roi,
        //     2.0,
        //     10.0,
        //     20.0f32.to_radians(),
        // );

        let camera_matrix = context.camera_matrix;
        let transformed_roi_points: Vec<_> = filtered_roi_points
            .iter()
            .map(|&p| camera_matrix.pixel_to_ground(p))
            .flatten()
            .collect();

        // let new_center_pixel = ellifit::<Ground>(&transformed_roi_points)
        //     .ok()
        //     .and_then(|fitted_ellipse_ground| {
        //         println!("fitted_ellipse_ground: {:?}", fitted_ellipse_ground);
        //         camera_matrix
        //             .ground_to_pixel(fitted_ellipse_ground.center)
        //             .ok()
        //             .filter(|p| line.squared_distance_to(*p) < min_distance_from_center_squared)
        //     })
        //     .unwrap_or(cleaned_center);
        // let new_center_pixel = ellifit::<Pixel>(&roi_points)
        //     .ok()
        //     .and_then(|fitted_ellipse_ground| {
        //         println!("fitted_ellipse_ground: {:?}", fitted_ellipse_ground);
        //         Some(fitted_ellipse_ground.center)
        //     })
        //     .unwrap_or(cleaned_center);

        (
            CenterCirclePoints {
                center: cleaned_center,
                // center: new_center_pixel,
                // points: arc_clusters.iter().flatten().copied().collect(),
                points: filtered_roi_points,
                // points: center_circle.used_points_original.clone(),
            },
            LineSegment(
                line.point,
                line.point + line.direction, // line.point - (line_length_half * line.direction.inner).framed(),
            ),
            source_lines
                .iter()
                .map(|line| {
                    let d = line.direction * 150.0;
                    let p = line.closest_point(circle_center);
                    LineSegment(p - d, p + d)
                })
                .collect_vec(),
        )
    })
}

fn get_center_circle_line(
    circle_center: Point2<Pixel>,
    context: &CycleContext,
    roi: Rectangle<Pixel>,
    roi_points: &[Point2<Pixel>],
    min_distance_from_center_squared: f32,
) -> Option<(Line2<Pixel>, Vec<Line2<Pixel>>)> {
    let lines = get_hough_line_with_edges_imgproc(
        roi_points,
        Some(roi),
        &HoughParams {
            peak_threshold: *context.refine_hough_threshold as u32,
            rho_bin_size: *context.refine_hough_rho_bin_size,
            suppression_radius: *context.refine_hough_nms_radius,
        },
    );
    {
        // Ascending order
        if lines.len() >= 2 {
            let first = lines.first().unwrap();
            let last = lines.last().unwrap();
            assert!(first.1 <= last.1);
        }
    }
    let clustering_max_line_to_line_distance = 5.0;
    let clustering_direction_cosine_similarity = (5.0f32).to_radians().cos();
    let middle_and_source_lines = match lines.len() {
        0 => None,
        1 => lines.first().map(|(line, _)| (*line, vec![*line])),
        _ => {
            let mut clusters: Vec<Vec<(_, _)>> = vec![];
            let mut remaining_lines: Vec<_> = lines
                .iter()
                .copied()
                .filter(|(line, score)| {
                    line.squared_distance_to(circle_center) < min_distance_from_center_squared
                })
                .collect();

            if remaining_lines.is_empty() {
                return None;
            }
            while let Some((chosen_line, score)) = remaining_lines.pop() {
                let mut current_cluster = vec![(chosen_line, score)];
                if remaining_lines.len() < 2 {
                    clusters.push(current_cluster);
                    continue;
                }

                remaining_lines[..remaining_lines.len() - 1]
                    .iter()
                    .for_each(|(line2, score)| {
                        if chosen_line.direction.dot(line2.direction).abs()
                            >= clustering_direction_cosine_similarity
                            && chosen_line.distance_to(line2.point)
                                <= clustering_max_line_to_line_distance
                        {
                            current_cluster.push((*line2, *score));
                        }
                    });
                clusters.push(current_cluster);
            }

            let clustered_lines: Vec<_> = clusters
                .iter()
                .map(|cluster| {
                    let (merged_point, merged_direction, merged_point_count) = cluster.iter().fold(
                        (point![0.0, 0.0], vector![0.0, 0.0], 0),
                        |accum, (line, point_count)| {
                            (
                                accum.0 + line.closest_point(circle_center).coords(),
                                accum.1 + line.direction,
                                accum.2 + point_count,
                            )
                        },
                    );

                    (
                        Line2::<Pixel> {
                            point: merged_point / cluster.len() as f32,
                            direction: merged_direction / cluster.len() as f32,
                        },
                        merged_point_count,
                    )
                })
                .collect();
            let best_line = clustered_lines
                .iter()
                .max_by_key(|(_, count)| *count)
                .map(|(line, _)| *line)
                .unwrap();

            Some((
                best_line,
                clustered_lines
                    .into_iter()
                    .map(|(line, _)| line)
                    .collect_vec(),
            ))
        }
    };
    middle_and_source_lines
}

// fn refine_2(
//     center_circle: &RansacResultCircleWithTransformation<Pixel, Ground>,
//     unused_points: &[Point2<Pixel>],
//     context: &CycleContext,
// ) -> Vec<(CenterCirclePoints<Pixel>, Vec<LineSegment<Pixel>>, f32)> {
//     if center_circle.used_points_original.len() < 3 {
//         return vec![];
//     }
//     // TODO duplicate code!
//     let circle_points_pixel = &center_circle.used_points_original;
//     let roi_padding = 10.0;
//     let roi = get_center_circle_roi(circle_points_pixel, (roi_padding, roi_padding));
//     let roi_x_range = roi.min.x()..=roi.max.x();
//     let roi_y_range = roi.min.y()..=roi.max.y();
//     let roi_height = roi.max.y() - roi.min.y();
//     let roi_width = roi.max.x() - roi.min.x();

//     let roi_points = unused_points
//         .into_iter()
//         .filter(|p| roi_x_range.contains(&p.x()) && roi_y_range.contains(&p.y()));

// }

fn circle_circumference_percentage_filter(
    circle_center: Point2<Ground>,
    circle_points: &[Point2<Ground>],
    minimum_circumference_occupancy_ratio: f32,
) -> bool {
    const DEFAULT_BIN_COUNT: usize = 66;
    let bin_bount = if circle_points.len() / 2 < DEFAULT_BIN_COUNT {
        circle_points.len() / 2
    } else {
        DEFAULT_BIN_COUNT
    };
    let angle_to_bin_indice_factor = PI * 2.0 / (bin_bount as f32);
    let filled_bin_count = circle_points
        .iter()
        .map(|point| {
            let angle = (circle_center.y() - point.y()).atan2(circle_center.x() - point.x());

            (angle / angle_to_bin_indice_factor).ceil() as i32
        })
        .unique()
        .count();

    let percentage = filled_bin_count as f32 / bin_bount as f32;

    percentage >= minimum_circumference_occupancy_ratio.clamp(0.0, 1.0)
}

fn get_arc_clusters(
    center: Point2<Pixel>,
    points: &[Point2<Pixel>],
    roi: Rectangle<Pixel>,
    direct_inclusion_distance: f32,
    max_distance: f32,
    max_angle_deviation: f32,
) -> Vec<Vec<Point2<Pixel>>> {
    let shape = roi.max - roi.min;

    if shape.y() == 0.0 {
        return vec![];
    }
    // make the ROI a square -> the points will be circularly distributed, making angle based calculaions easier
    let aspect_ratio = shape.x() / shape.y();
    let (scaled_center_x, scaled_center_y) = (center.x(), center.y() * aspect_ratio);

    let mut sorted_points: Vec<(_, _)> = points
        .into_iter()
        .map(|v| {
            let diff_x = v.x() - scaled_center_x;
            let diff_y = v.y() * aspect_ratio - scaled_center_y;
            (v, diff_x.atan2(diff_y))
        })
        .sorted_unstable_by_key(|(_, angle)| (angle.to_degrees() * 4.0) as i16)
        .collect();

    let point_count = points.len();
    let mut clusters = Vec::with_capacity((point_count / 4).min(4));

    let time = SystemTime::now();
    let mut iterations = 0;

    while sorted_points.len() > 0 {
        let mut data_a = sorted_points.pop().map(|d| (*d.0, d.1)).unwrap();
        let (mut current_cluster, remainder): (Vec<_>, Vec<_>) =
            sorted_points.into_iter().partition(|(&point_b, angle_b)| {
                let (point_a, angle_a) = data_a;
                let point_to_point_distance = distance(point_a, point_b);

                let main = point_to_point_distance <= direct_inclusion_distance;
                let secondary = point_to_point_distance < max_distance
                    && (angle_a - angle_b).abs() <= max_angle_deviation;

                let good = main || secondary;
                if good {
                    data_a = (point_b, *angle_b);
                }
                good
            });
        if !current_cluster.is_empty() {
            current_cluster.push((&data_a.0, data_a.1));
            clusters.push(current_cluster.into_iter().map(|v| *v.0).collect());
        }
        sorted_points = remainder;
        iterations += 1;

        if sorted_points.len() as f32 > point_count as f32 * 0.1 {
            break;
        }
    }
    println!(
        "total: {point_count}, iterations: {iterations}, clusters: {} time: {:?}",
        clusters.len(),
        time.elapsed()
    );
    clusters
}

fn detect_and_filter_circles(
    edge_points: &[Point2<Pixel>],
    context: &CycleContext,
    y_exclusion_threshold: u32,
) -> Vec<(CenterCirclePoints<Pixel>, Vec<LineSegment<Pixel>>, f32)> {
    let camera_matrix = context.camera_matrix;
    let transformer =
        |pixel_coordinates: &Point2<Pixel>| camera_matrix.pixel_to_ground(*pixel_coordinates).ok();
    let mut rng = ChaChaRng::from_entropy();
    let mut ransac = RansacCircleWithTransformation::<Pixel, Ground>::new(
        context.field_dimensions.center_circle_diameter / 2.0,
        *context.ransac_circle_inlier_threshold,
        edge_points.to_vec(),
        transformer,
        None,
        context.ransac_sample_size_percentage.copied(),
    );
    let input_point_count = edge_points.len();
    let ransac_iterations = *context.ransac_iterations;
    let ransac_circle_minimum_circumference_percentage =
        *context.ransac_circle_minimum_circumference_percentage;
    (0..*context.ransac_maximum_number_of_circles)
        .filter_map(|_| {
            ransac
                .next_candidate(&mut rng, ransac_iterations)
                .and_then(|result| {
                    let circle = result.circle;
                    let used_points_transformed = &result.used_points_transformed;

                    let y_range = y_exclusion_threshold..camera_matrix.image_size.y() as u32;
                    camera_matrix
                        .ground_to_pixel(circle.center)
                        .ok()
                        .and_then(|circle_center_px| {
                            let center_tr = ellifit(used_points_transformed)
                                .map_or(circle.center, |e| e.center);
                            if y_range.contains(&(circle_center_px.y() as u32))
                                && circle_circumference_percentage_filter(
                                    center_tr,
                                    used_points_transformed,
                                    ransac_circle_minimum_circumference_percentage,
                                )
                            {
                                refine_center_circle(
                                    &result,
                                    circle_center_px,
                                    &ransac.unused_points_original,
                                    // edge_points,
                                    500,
                                    context,
                                )
                                .map(|v| (v.0, v.2, result.score))
                            } else {
                                None
                            }
                        })
                })
        })
        .sorted_by_key(|value| input_point_count - value.0.points.len())
        .collect()
}

fn get_edges_from_canny_edge_detection(
    context: &CycleContext,
    y_exclusion_threshold: u32,
) -> Vec<Point2<Pixel>> {
    let canny_source_type = if *context.preprocessing_luma_without_difference {
        EdgeSourceType::LumaOfYCbCr
    } else {
        EdgeSourceType::DifferenceOfGrayAndRgbRange
    };

    get_edges_canny(
        *context.preprocessing_gaussian_sigma,
        *context.canny_low_threshold,
        *context.canny_high_threshold,
        context.image,
        canny_source_type,
        Some(y_exclusion_threshold),
    )

    // get_edges_canny_imageproc(
    //     *context.preprocessing_gaussian_sigma,
    //     *context.canny_low_threshold,
    //     *context.canny_high_threshold,
    //     context.image,
    //     canny_source_type,
    //     Some(y_exclusion_threshold),
    // )
}

fn get_edges_from_segments(
    filtered_segments: &FilteredSegments,
    upper_points_exclusion_threshold_y: Option<u32>,
) -> Vec<Point2<Pixel>> {
    let y_exclusion_threshold: f32 = upper_points_exclusion_threshold_y.unwrap_or_default() as f32;

    filtered_segments
        .scan_grid
        .vertical_scan_lines
        .iter()
        .flat_map(|scan_line| {
            let scan_line_position = scan_line.position;
            scan_line
                .segments
                .iter()
                .filter_map(move |segment| -> Option<[Point2<Pixel>; 2]> {
                    let center = (segment.start + segment.end) as f32 / 2.0;
                    if center > y_exclusion_threshold {
                        Some([
                            point![scan_line_position as f32, segment.start as f32],
                            point![scan_line_position as f32, segment.end as f32],
                        ])
                    } else {
                        None
                    }
                })
                .flatten()
        })
        .collect()
}

fn get_y_exclusion_threshold(context: &CycleContext) -> u32 {
    context
        .camera_matrix
        .horizon
        .map_or(0, |h| h.horizon_y_minimum() as u32)
}
