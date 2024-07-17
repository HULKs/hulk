use std::{
    f32::consts::PI,
    time::{Duration, Instant},
};

use color_eyre::Result;
use itertools::Itertools;
use rand::SeedableRng;
use rand_chacha::ChaChaRng;
use serde::{Deserialize, Serialize};

use calibration::center_circle::circle_points::CenterCirclePoints;
use context_attribute::context;
use coordinate_systems::{Ground, Pixel};
use edge_detection::{get_edge_image_canny, EdgeSourceType};
use framework::{deserialize_not_implemented, AdditionalOutput, MainOutput};
use linear_algebra::{point, Point2};
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
        );

        let elapsed_time_after_all_processing = processing_start.elapsed();

        context
            .detected_edge_points
            .fill_if_subscribed(|| filtered_points);

        context.circles_points_pixel_scores.fill_if_subscribed(|| {
            filtered_calibration_circles_ground
                .iter()
                .map(|ransac_result| ransac_result.score)
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

        let center_circle_points: Option<CenterCirclePoints<Pixel>> =
            filtered_calibration_circles_ground
                .first()
                .map(|ransac_result| CenterCirclePoints {
                    center: context
                        .camera_matrix
                        .ground_to_pixel(ransac_result.circle.center)
                        .expect("ground -> pixel failed"),
                    points: ransac_result.used_points_original.clone(),
                });

        Ok(MainOutputs {
            calibration_center_circle: CalibrationFeatureDetectorOutput {
                detected_feature: center_circle_points,
                cycle_skipped: false,
            }
            .into(),
        })
    }
}

fn circle_circumference_percentage_filter(
    circle_center: Point2<Ground>,
    circle_points: &[Point2<Ground>],
    minimum_circumference_occupancy_ratio: f32,
) -> bool {
    const DEFAULT_BIN_COUNT: usize = 100;
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
) -> Vec<RansacResultCircleWithTransformation<Pixel, Ground>> {
    let transformer =
        |pixel_coordinates: &Point2<Pixel>| camera_matrix.pixel_to_ground(*pixel_coordinates).ok();
    let mut rng = ChaChaRng::from_entropy();
    let mut ransac = RansacCircleWithTransformation::<Pixel, Ground>::new(
        target_circle_radius,
        ransac_circle_inlier_threshold,
        edge_points.to_vec(),
        transformer,
    );
    let input_point_count = edge_points.len();

    (0..maximum_number_of_circles)
        .filter_map(|_| ransac.next_candidate(&mut rng, ransac_iterations))
        .filter(|result| {
            let circle = result.circle;
            let used_points_transformed = &result.used_points_transformed;
            let max_y = camera_matrix.image_size.y();
            camera_matrix
                .ground_to_pixel(circle.center)
                .is_ok_and(|center| {
                    center.y() <= max_y
                        && circle_circumference_percentage_filter(
                            circle.center,
                            used_points_transformed,
                            ransac_circle_minimum_circumference_percentage,
                        )
                })
        })
        .sorted_by_key(|value| input_point_count - value.used_points_original.len())
        .collect()
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

fn get_edges_from_canny_edge_detection(context: &CycleContext) -> Vec<Point2<Pixel>> {
    let canny_source_type = if *context.preprocessing_luma_without_difference {
        EdgeSourceType::LumaOfYCbCr
    } else {
        EdgeSourceType::DifferenceOfGrayAndRgbRange
    };
    let edge_image = get_edge_image_canny(
        *context.preprocessing_gaussian_sigma,
        *context.canny_low_threshold,
        *context.canny_high_threshold,
        context.image,
        canny_source_type,
    );

    let y_exclusion_threshold = context
        .camera_matrix
        .horizon
        .map_or(0, |h| h.horizon_y_minimum() as u32);

    edge_image
        .enumerate_pixels()
        .filter_map(|(x, y, color)| {
            if color[0] > 127 && y > y_exclusion_threshold {
                Some(point![x as f32, y as f32])
            } else {
                None
            }
        })
        .collect()
}
