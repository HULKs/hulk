use std::{
    f32::consts::PI,
    time::{Duration, Instant},
};

use crate::image_ops::{generate_luminance_image, gray_image_to_hulks_grayscale_image};
use color_eyre::Result;
use context_attribute::context;
use coordinate_systems::{Ground, Pixel};
use fast_image_resize::FilterType;
use framework::{deserialize_not_implemented, AdditionalOutput, MainOutput};
use image::{GrayImage, Luma, RgbImage};
use imageproc::{edges::canny, filter::gaussian_blur_f32, map::map_colors};
use itertools::Itertools;
use linear_algebra::{distance, point, Point2};
use projection::{camera_matrix::CameraMatrix, Projection};
use rand::SeedableRng;
use rand_chacha::ChaChaRng;
use ransac::circles::circle_ransac::{
    RansacCircleWithTransformation, RansacResultCircleWithTransformation,
};
use serde::{Deserialize, Serialize};
use types::{
    field_dimensions::FieldDimensions, filtered_segments::FilteredSegments,
    grayscale_image::GrayscaleImage, ycbcr422_image::YCbCr422Image,
};

#[derive(Deserialize, Serialize)]
pub struct CalibrationLineDetection {
    #[serde(skip, default = "deserialize_not_implemented")]
    last_processed_instance: Instant,
}

#[context]
pub struct CreationContext {}
#[context]
pub struct CycleContext {
    enable: Parameter<bool, "calibration_line_detection.$cycler_instance.enable">,
    canny_low_threshold: Parameter<f32, "calibration_line_detection.canny_low_threshold">,
    canny_high_threshold: Parameter<f32, "calibration_line_detection.canny_high_threshold">,
    gaussian_sigma: Parameter<f32, "calibration_line_detection.gaussian_sigma">,
    maximum_number_of_circles:
        Parameter<usize, "calibration_line_detection.maximum_number_of_circles">,
    ransac_iterations: Parameter<usize, "calibration_line_detection.ransac_iterations">,
    ransac_circle_inlier_threshold:
        Parameter<f32, "calibration_line_detection.ransac_circle_inlier_threshold">,
    ransac_circle_minimum_circumference_percentage:
        Parameter<f32, "calibration_line_detection.ransac_circle_minimum_circumference_percentage">,
    debug_image_resized_width:
        Parameter<u32, "calibration_line_detection.debug_image_resized_width">,
    run_next_cycle_after_ms: Parameter<u64, "calibration_line_detection.run_next_cycle_after_ms">,
    // Heavier calculation due to rgb conversion
    skip_rgb_based_difference_image:
        Parameter<bool, "calibration_line_detection.skip_rgb_based_difference_image">,
    get_edges_from_segments: Parameter<bool, "calibration_line_detection.get_edges_from_segments">,

    field_dimensions: Parameter<FieldDimensions, "field_dimensions">,

    // TODO activate this once calibration controller can emit this value
    // pub camera_position_of_calibration_lines_request:
    //     RequiredInput<Option<CameraPosition>, "requested_calibration_lines?">,
    image: Input<YCbCr422Image, "image">,
    camera_matrix: RequiredInput<Option<CameraMatrix>, "camera_matrix?">,
    filtered_segments: Input<FilteredSegments, "filtered_segments">,

    difference_image:
        AdditionalOutput<Option<GrayscaleImage>, "calibration_line_detection.difference_image">,
    blurred_image:
        AdditionalOutput<Option<GrayscaleImage>, "calibration_line_detection.blurred_image">,
    detected_edge_points:
        AdditionalOutput<Vec<Point2<Pixel>>, "calibration_line_detection.detected_edge_points">,

    timings_for_steps_ms:
        AdditionalOutput<Vec<(String, u128)>, "calibration_line_detection.timings_for_steps">,
    cycle_time: AdditionalOutput<Duration, "calibration_line_detection.cycle_time">,
    circles_points_pixel: AdditionalOutput<
        Vec<(Point2<Pixel>, Vec<Point2<Pixel>>)>,
        "calibration_line_detection.circles_points_pixel",
    >,
    circles_points_pixel_scores:
        AdditionalOutput<Vec<f32>, "calibration_line_detection.circles_points_pixel_scores">,
}

#[context]
#[derive(Default)]
pub struct MainOutputs {
    pub detected_calibration_circles: MainOutput<Vec<(Point2<Ground>, Vec<Point2<Ground>>)>>,
}

impl CalibrationLineDetection {
    pub fn new(_context: CreationContext) -> Result<Self> {
        Ok(Self {
            last_processed_instance: Instant::now(),
        })
    }

    pub fn cycle(&mut self, mut context: CycleContext) -> Result<MainOutputs> {
        if !context.enable
            || (self.last_processed_instance.elapsed()
                < Duration::from_millis(*context.run_next_cycle_after_ms))
        {
            // TODO activate the below part after the calibration controller can emit the request
            // || context.camera_position_of_calibration_lines_request != context.camera_position {
            return Ok(MainOutputs {
                detected_calibration_circles: vec![].into(),
            });
        }

        let processing_start = Instant::now();

        let (filtered_points, debug_images, mut edges_timing) = if *context.get_edges_from_segments
        {
            let edges = get_edges_from_segments(
                context.filtered_segments,
                context
                    .camera_matrix
                    .horizon
                    .map(|h| h.horizon_y_minimum() as u32),
            );

            (
                edges,
                None,
                vec![(
                    "edges_from_segments_us".to_owned(),
                    processing_start.elapsed().as_micros(),
                )],
            )
        } else {
            get_edges_canny(&context)
        };

        let elapsed_time_after_getting_edges = processing_start.elapsed();
        let detected_circles_and_results = detect_circles(
            &filtered_points,
            context.camera_matrix,
            *context.maximum_number_of_circles,
            *context.ransac_iterations,
            *context.ransac_circle_inlier_threshold,
            context.field_dimensions.center_circle_diameter / 2.0,
            context
                .field_dimensions
                .length
                .max(context.field_dimensions.width)
                / 2.0,
        );

        let filtered_calibration_circles_ground =
            filter_circles(detected_circles_and_results, &context);
        let elapsed_time_after_all_processing = processing_start.elapsed();

        if let Some(blurred) = debug_images {
            let debug_image_size = {
                let expected_width = *context.debug_image_resized_width;
                if expected_width >= context.image.width() {
                    None
                } else {
                    let aspect_ratio = context.image.height() as f32 / context.image.width() as f32;

                    Some((
                        expected_width,
                        (expected_width as f32 * aspect_ratio) as u32,
                    ))
                }
            };

            context.blurred_image.fill_if_subscribed(|| {
                Some(gray_image_to_hulks_grayscale_image(
                    &blurred,
                    debug_image_size,
                    Some(FilterType::Box),
                ))
            });
        } else {
            // Start filling outputs
            context.difference_image.fill_if_subscribed(|| None);
            context.blurred_image.fill_if_subscribed(|| None);
        }
        context
            .detected_edge_points
            .fill_if_subscribed(|| filtered_points);
        context.circles_points_pixel.fill_if_subscribed(|| {
            filtered_calibration_circles_ground
                .iter()
                .map(|ransac_result| {
                    (
                        context
                            .camera_matrix
                            .ground_to_pixel(ransac_result.circle.center)
                            .expect("pixel -> ground -> pixel: last failed"),
                        ransac_result
                            .used_points_transformed
                            .iter()
                            .map(|point| {
                                context
                                    .camera_matrix
                                    .ground_to_pixel(*point)
                                    .expect("pixel -> ground -> pixel: last failed")
                            })
                            .collect_vec(),
                    )
                })
                .collect_vec()
        });
        context.circles_points_pixel_scores.fill_if_subscribed(|| {
            filtered_calibration_circles_ground
                .iter()
                .map(|ransac_result| ransac_result.score)
                .collect_vec()
        });

        context
            .cycle_time
            .fill_if_subscribed(|| elapsed_time_after_all_processing);
        context.timings_for_steps_ms.fill_if_subscribed(|| {
            let rest_of_time = vec![
                (
                    "circle_us".to_string(),
                    (elapsed_time_after_all_processing - elapsed_time_after_getting_edges)
                        .as_micros(),
                ),
                (
                    "elapsed_time_after_all_processing_ms".to_string(),
                    (elapsed_time_after_all_processing).as_millis(),
                ),
            ];

            edges_timing.extend(rest_of_time);
            edges_timing
        });

        // Set this as late as possible, to execute the next rount at least after the configured delay (checked at the beginning)
        self.last_processed_instance = Instant::now();

        Ok(MainOutputs {
            detected_calibration_circles: filtered_calibration_circles_ground
                .into_iter()
                .map(|v| (v.circle.center, v.used_points_transformed))
                .collect_vec()
                .into(),
        })
    }
}

fn filter_circles(
    detected_circles_and_results: Vec<RansacResultCircleWithTransformation<Pixel, Ground>>,
    context: &CycleContext,
) -> Vec<RansacResultCircleWithTransformation<Pixel, Ground>> {
    detected_circles_and_results
        .into_iter()
        .filter(|result| {
            let circle = result.circle;
            let used_points_transformed = &result.used_points_transformed;
            let max_y = context.camera_matrix.image_size.y();
            context
                .camera_matrix
                .ground_to_pixel(circle.center)
                .is_ok_and(|center| {
                    center.y() <= max_y
                        && circle_circumference_percentage_filter(
                            circle.center,
                            used_points_transformed,
                            *context.ransac_circle_minimum_circumference_percentage,
                        )
                })
        })
        .collect_vec()
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
                .filter_map(move |segment| -> Option<Point2<Pixel>> {
                    let center = (segment.start + segment.end) as f32 / 2.0;
                    if center > y_exclusion_threshold {
                        Some(point![scan_line_position as f32, center])
                    } else {
                        None
                    }
                })
        })
        .collect_vec()
}

fn get_edges_canny(
    context: &CycleContext,
) -> (
    Vec<Point2<Pixel>>,
    Option<image::ImageBuffer<Luma<u8>, Vec<u8>>>,
    Vec<(String, u128)>,
) {
    let processing_start = Instant::now();

    let difference = {
        if *context.skip_rgb_based_difference_image {
            generate_luminance_image(context.image, None).expect("Generating luma image failed")
        } else {
            let rgb = RgbImage::from(context.image);

            let difference = rgb_image_to_difference(&rgb);

            GrayImage::from_vec(
                difference.width(),
                difference.height(),
                difference.into_vec(),
            )
            .expect("GrayImage construction after resize failed")
        }
    };
    let elapsed_time_after_difference = processing_start.elapsed();

    let blurred = gaussian_blur_f32(&difference, *context.gaussian_sigma);
    let elapsed_time_after_blurred = processing_start.elapsed();

    let edges = canny(
        &blurred,
        *context.canny_low_threshold,
        *context.canny_high_threshold,
    );
    let elapsed_time_after_edges = processing_start.elapsed();

    let filtered_points = get_filtered_edge_points(
        &edges,
        context
            .camera_matrix
            .horizon
            .map(|h| h.horizon_y_minimum() as u32),
    );
    let elapsed_time_after_filtering_edges = processing_start.elapsed();

    let timing_information = vec![
        (
            "difference_ms".to_string(),
            elapsed_time_after_difference.as_millis(),
        ),
        (
            "blurred_ms".to_string(),
            (elapsed_time_after_blurred - elapsed_time_after_difference).as_millis(),
        ),
        (
            "edges_ms".to_string(),
            (elapsed_time_after_edges - elapsed_time_after_blurred).as_millis(),
        ),
        (
            "edges_filtering_us".to_string(),
            (elapsed_time_after_filtering_edges - elapsed_time_after_edges).as_micros(),
        ),
    ];
    (filtered_points, Some(blurred), timing_information)
}

pub fn rgb_image_to_difference(rgb: &RgbImage) -> GrayImage {
    map_colors(rgb, |color| {
        Luma([
            (rgb_pixel_to_luminance(&color) as i16 - rgb_pixel_to_difference(&color) as i16)
                .clamp(0, 255) as u8,
        ])
    })
}

pub fn rgb_pixel_to_luminance(rgb: &image::Rgb<u8>) -> f32 {
    (rgb[0] as f32 + rgb[1] as f32 + rgb[2] as f32) / 3.0
}

pub fn rgb_pixel_to_difference(rgb: &image::Rgb<u8>) -> u8 {
    let minimum = rgb.0.iter().min().unwrap();
    let maximum = rgb.0.iter().max().unwrap();
    maximum - minimum
}

fn get_filtered_edge_points(
    edges: &GrayImage,
    upper_points_exclusion_threshold_y: Option<u32>,
) -> Vec<Point2<Pixel>> {
    let y_exclusion_threshold: u32 = upper_points_exclusion_threshold_y.unwrap_or_default();
    edges
        .enumerate_pixels()
        .filter_map(|(x, y, color)| {
            if color[0] > 127 && y > y_exclusion_threshold {
                Some(point![x as f32, y as f32])
            } else {
                None
            }
        })
        .collect_vec()
}

fn circle_circumference_percentage_filter(
    circle_center: Point2<Ground>,
    circle_points: &[Point2<Ground>],
    circumference_percentage: f32,
) -> bool {
    let slices = 100;
    // Locations 0 to 100, also could be angle in degree or whatever
    // atan() -> [-PI/2, PI/2]
    let angle_to_slice_indice_factor = PI * 2.0 / (slices as f32);

    let present_slice_count = circle_points
        .iter()
        .map(|point| {
            let angle = (circle_center.y() - point.y()).atan2(circle_center.x() - point.x());

            (angle / angle_to_slice_indice_factor).ceil() as i32
        })
        .unique()
        .count();

    let current_percentage = present_slice_count as f32 / slices as f32;

    current_percentage >= circumference_percentage.clamp(0.0, 1.0)
}

#[allow(clippy::too_many_arguments)]
fn detect_circles(
    edge_points: &[Point2<Pixel>],
    camera_matrix: &CameraMatrix,
    maximum_number_of_circles: usize,
    ransac_iterations: usize,
    ransac_circle_inlier_threshold: f32,
    target_circle_radius: f32,
    center_distance_penalty_threshold: f32,
) -> Vec<RansacResultCircleWithTransformation<Pixel, Ground>> {
    let transformer = |pixel_points: &[Point2<Pixel>]| {
        pixel_points
            .iter()
            .filter_map(|pixel_coordinates| {
                let point = camera_matrix.pixel_to_ground(*pixel_coordinates);
                point.ok().and_then(|point| {
                    if distance(point, Point2::origin()) <= center_distance_penalty_threshold {
                        Some(point)
                    } else {
                        None
                    }
                })
            })
            .collect_vec()
    };
    let mut rng = ChaChaRng::from_entropy();
    let mut ransac = RansacCircleWithTransformation::<Pixel, Ground>::new(
        target_circle_radius,
        ransac_circle_inlier_threshold,
        edge_points.to_vec(),
        transformer,
        &mut rng,
    );
    let input_point_count = edge_points.len();
    (0..maximum_number_of_circles)
        .filter_map(|_| ransac.next_candidate(&mut rng, ransac_iterations))
        .sorted_by_key(|value| input_point_count - value.used_points.len())
        .collect_vec()
}
