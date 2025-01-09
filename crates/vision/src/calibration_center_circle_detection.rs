use std::{
    f32::consts::PI,
    time::{Duration, Instant},
};

use color_eyre::Result;
use edge_detection::{get_edges_canny, EdgeSourceType};
use geometry::{line::Line2, line_segment::LineSegment, rectangle::Rectangle, Distance};
use itertools::Itertools;
use rand::SeedableRng;
use rand_chacha::ChaChaRng;
use serde::{Deserialize, Serialize};

use calibration::center_circle::circle_points::CenterCirclePoints;
use context_attribute::context;
use coordinate_systems::{Ground, Pixel};

use framework::{deserialize_not_implemented, AdditionalOutput, MainOutput};
use linear_algebra::{point, vector, IntoFramed, Point2};
use projection::{camera_matrix::CameraMatrix, Projection};
use ransac::circles::circle::{
    RansacCircleWithTransformation, RansacResultCircleWithTransformation,
};
use types::{
    calibration::{CalibrationCommand, CalibrationFeatureDetectorOutput},
    camera_position::CameraPosition,
    field_dimensions::FieldDimensions,
    filtered_segments::FilteredSegments,
    ycbcr422_image::YCbCr422Image,
};

use crate::hough::get_center_circle_roi;

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

        let filtered_points = if *context.preprocessing_get_edges_from_segments {
            get_edges_from_segments(
                context.filtered_segments,
                context
                    .camera_matrix
                    .horizon
                    .map(|h| h.horizon_y_minimum() as u32),
            )
        } else {
            get_edges_from_canny_edge_detection(&context)
        };

        let elapsed_time_after_getting_edges = processing_start.elapsed();
        let filtered_calibration_circles_ground = detect_and_filter_circles(
            &filtered_points,
            context.camera_matrix,
            *context.ransac_maximum_number_of_circles,
            *context.ransac_iterations,
            *context.ransac_circle_inlier_threshold,
            context.field_dimensions.center_circle_diameter / 2.0,
            *context.ransac_circle_minimum_circumference_percentage,
            None,
            context.ransac_sample_size_percentage.copied(),
        );

        let elapsed_time_after_all_processing = processing_start.elapsed();

        context.circle_lines.fill_if_subscribed(|| {
            filtered_calibration_circles_ground
                .iter()
                .map(|(_, line, _)| *line)
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

fn get_center_circle_lines(
    center_circle: &RansacResultCircleWithTransformation<Pixel, Ground>,
    circle_center: Point2<Pixel>,
    ransac_source_points: &[Point2<Pixel>],
    ransac_iters: usize,
) -> Option<(CenterCirclePoints<Pixel>, LineSegment<Pixel>)> {
    if center_circle.used_points_original.len() < 3 {
        return None;
    }
    let circle_points_pixel = &center_circle.used_points_original;
    let roi = get_center_circle_roi(circle_points_pixel, (5.0, 5.0));

    let roi_points: Vec<_> = ransac_source_points
        .iter()
        .filter(|point| {
            point.x() >= roi.min.x()
                && point.x() <= roi.max.x()
                && point.y() >= roi.min.y()
                && point.y() <= roi.max.y()
        })
        .cloned()
        .collect();

    let maximum_score_distance = 10.0;
    let maximum_inclusion_distance = 10.0;
    let mut line_ransac = ransac::Ransac::new(roi_points.clone());
    let mut rng = ChaChaRng::from_entropy();
    // fn best_fit_line(points: &[Point2<Pixel>]) -> LineSegment<Pixel> {
    //     let half_size = points.len() / 2;
    //     let line_start = find_center_of_group(&points[0..half_size]);
    //     let line_end = find_center_of_group(&points[half_size..points.len()]);

    //     LineSegment(line_start, line_end)
    // }
    // fn find_center_of_group(group: &[Point2<Pixel>]) -> Point2<Pixel> {
    //     group
    //         .iter()
    //         .map(|point| point.coords())
    //         .sum::<Vector2<_>>()
    //         .unscale(group.len() as f32)
    //         .as_point()
    // }

    let distance_threshold = ((roi.max.coords() - roi.min.coords()).norm() * 0.3).max(20.0);

    let lines = (0..3)
        .map(|_| {
            line_ransac.next_line(
                &mut rng,
                ransac_iters,
                maximum_score_distance,
                maximum_inclusion_distance,
            )
        })
        .flat_map(|result| {
            if let Some(ransac_line) = result.line {
                let distance = ransac_line.squared_distance_to(circle_center);
                if distance < distance_threshold {
                    Some((
                        result.used_points,
                        ransac_line,
                        ransac_line.squared_distance_to(circle_center),
                    ))
                } else {
                    None
                }
            } else {
                None
            }
        })
        .sorted_by(|a, b| a.2.total_cmp(&b.2))
        .collect_vec();

    // Construct the center line by clustering all lines
    let thresh = 0.6;
    let best_line = match lines.len() {
        0 => None,
        1 => lines.first().map(|(_used_points, line, _distance)| *line),
        _ => lines.first().map(|(_used_points, first_line, _distance)| {
            let selected_lines_and_weights = lines
                .iter()
                .filter(|(_used_points2, _line, _)| {
                    first_line.direction.dot(_line.direction) > thresh
                })
                .map(|(_used_points2, line, _)| (line.closest_point(circle_center), line.direction))
                .collect_vec();

            let mut accum_point = point![0.0f32, 0.0f32];
            let mut accum_direction = vector![0.0, 0.0];
            for (point, direction) in selected_lines_and_weights {
                accum_point += point.coords();
                accum_direction += direction;
            }

            Line2::<Pixel> {
                point: accum_point,
                direction: accum_direction,
            }
        }),
    };

    let point_distance = 10.0;

    // TODO Might be better to filter and combine the already split points by ransac
    best_line.map(|line| {
        let filtered_roi_points = roi_points
            .iter()
            .filter(|&&point| line.distance_to(point) > point_distance)
            .copied()
            .collect();

        (
            CenterCirclePoints {
                center: circle_center,
                points: filtered_roi_points,
            },
            LineSegment(
                line.point,
                line.point + line.direction, // line.point - (line_length_half * line.direction.inner).framed(),
            ),
        )
    })
}

fn circle_circumference_percentage_filter(
    circle_center: Point2<Ground>,
    circle_points: &[Point2<Ground>],
    minimum_circumference_occupancy_ratio: f32,
) -> bool {
    const DEFAULT_BIN_COUNT: usize = 36;
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

#[allow(clippy::too_many_arguments)]
fn detect_and_filter_circles(
    edge_points: &[Point2<Pixel>],
    camera_matrix: &CameraMatrix,
    maximum_number_of_circles: usize,
    ransac_iterations: usize,
    ransac_circle_inlier_threshold: f32,
    target_circle_radius: f32,
    ransac_circle_minimum_circumference_percentage: f32,
    ransac_circle_early_exit_fitting_score: Option<f32>,
    ransac_sample_size_percentage: Option<f32>,
) -> Vec<(CenterCirclePoints<Pixel>, LineSegment<Pixel>, f32)> {
    let transformer =
        |pixel_coordinates: &Point2<Pixel>| camera_matrix.pixel_to_ground(*pixel_coordinates).ok();
    let mut rng = ChaChaRng::from_entropy();
    let mut ransac = RansacCircleWithTransformation::<Pixel, Ground>::new(
        target_circle_radius,
        ransac_circle_inlier_threshold,
        edge_points.to_vec(),
        transformer,
        ransac_circle_early_exit_fitting_score,
        ransac_sample_size_percentage,
    );
    let input_point_count = edge_points.len();

    (0..maximum_number_of_circles)
        .filter_map(|_| {
            ransac
                .next_candidate(&mut rng, ransac_iterations)
                .and_then(|result| {
                    let circle = result.circle;
                    let used_points_transformed = &result.used_points_transformed;
                    let max_y = camera_matrix.image_size.y();
                    camera_matrix
                        .ground_to_pixel(circle.center)
                        .ok()
                        .and_then(|circle_center_px| {
                            if circle_center_px.y() <= max_y
                                && circle_circumference_percentage_filter(
                                    circle.center,
                                    used_points_transformed,
                                    ransac_circle_minimum_circumference_percentage,
                                )
                            {
                                get_center_circle_lines(&result, circle_center_px, edge_points, 200)
                                    .map(|v| (v.0, v.1, result.score))
                            } else {
                                None
                            }
                        })
                })
        })
        .sorted_by_key(|value| input_point_count - value.0.points.len())
        .collect()
}

fn get_edges_from_canny_edge_detection(context: &CycleContext) -> Vec<Point2<Pixel>> {
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
        Some(get_y_exclusion_threshold(context)),
    )
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
