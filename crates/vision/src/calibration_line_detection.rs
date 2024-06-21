use std::{
    f32::consts::{FRAC_PI_2, PI},
    time::{Duration, Instant},
};

use crate::image_ops::{generate_luminance_image, gray_image_to_hulks_grayscale_image};
use color_eyre::Result;
use context_attribute::context;
use coordinate_systems::{Ground, Pixel};
use fast_image_resize::FilterType;
use framework::{deserialize_not_implemented, AdditionalOutput, MainOutput};
use geometry::{circle::Circle, line::Line};
use image::{GrayImage, Luma, RgbImage};
use imageproc::{edges::canny, filter::gaussian_blur_f32, map::map_colors};
use itertools::Itertools;
use linear_algebra::{distance, point, Point2};
use nalgebra::ComplexField;
use ordered_float::Float;
use projection::{camera_matrix::CameraMatrix, Projection};
use rand::SeedableRng;
use rand_chacha::ChaChaRng;
use ransac::circles::circle_ransac::RansacCircleWithRadius;
use serde::{Deserialize, Serialize};
use types::{
    field_dimensions::FieldDimensions, grayscale_image::GrayscaleImage,
    ycbcr422_image::YCbCr422Image,
};

#[derive(Deserialize, Serialize)]
pub struct CalibrationLineDetection {
    #[serde(skip, default = "deserialize_not_implemented")]
    last_processed_instance: Instant,
    #[serde(skip, default = "deserialize_not_implemented")]
    random_state: ChaChaRng,
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

    field_dimensions: Parameter<FieldDimensions, "field_dimensions">,

    // TODO activate this once calibration controller can emit this value
    // pub camera_position_of_calibration_lines_request:
    //     RequiredInput<Option<CameraPosition>, "requested_calibration_lines?">,
    image: Input<YCbCr422Image, "image">,
    camera_matrix: RequiredInput<Option<CameraMatrix>, "camera_matrix?">,
    difference_image:
        AdditionalOutput<GrayscaleImage, "calibration_line_detection.difference_image">,
    blurred_image: AdditionalOutput<GrayscaleImage, "calibration_line_detection.blurred_image">,
    edges_image: AdditionalOutput<GrayscaleImage, "calibration_line_detection.edges_image">,
    timings_for_steps_ms:
        AdditionalOutput<Vec<(String, u128)>, "calibration_line_detection.timings_for_steps">,
    cycle_time: AdditionalOutput<Duration, "calibration_line_detection.cycle_time">,
    circles_and_used_points: AdditionalOutput<
        Vec<(Circle<Ground>, Point2<Pixel>, Vec<Point2<Pixel>>)>,
        "calibration_line_detection.circles_and_used_points",
    >,
}

#[context]
#[derive(Default)]
pub struct MainOutputs {
    pub detected_calibration_circles: MainOutput<Vec<Circle<Ground>>>,
}

impl CalibrationLineDetection {
    pub fn new(_context: CreationContext) -> Result<Self> {
        Ok(Self {
            last_processed_instance: Instant::now(),
            random_state: ChaChaRng::from_entropy(),
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

        let blurred = gaussian_blur_f32(&difference, *context.gaussian_sigma); // 2.0..10.0
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

        let elapsed_time_after_lines = processing_start.elapsed();

        let circles_and_used_points = detect_circles(
            &filtered_points,
            context.camera_matrix,
            *context.maximum_number_of_circles,
            *context.ransac_iterations,
            *context.ransac_circle_inlier_threshold,
            *context.ransac_circle_minimum_circumference_percentage,
            (context.field_dimensions.center_circle_diameter / 2.0)
                + (context.field_dimensions.line_width / 2.0),
        );

        let elapsed_time_after_circles = processing_start.elapsed();

        let calibration_circles = circles_and_used_points
            .clone()
            .into_iter()
            .map(|(circle, _ground_points, _used_points)| circle)
            .collect_vec();

        let elapsed_time_after_all_processing = processing_start.elapsed();

        context.difference_image.fill_if_subscribed(|| {
            gray_image_to_hulks_grayscale_image(
                &difference,
                debug_image_size,
                Some(FilterType::Box),
            )
        });
        context.blurred_image.fill_if_subscribed(|| {
            gray_image_to_hulks_grayscale_image(&blurred, debug_image_size, Some(FilterType::Box))
        });
        context.edges_image.fill_if_subscribed(|| {
            gray_image_to_hulks_grayscale_image(&edges, debug_image_size, Some(FilterType::Box))
        });

        context.circles_and_used_points.fill_if_subscribed(|| {
            circles_and_used_points
                .into_iter()
                .map(|(circle, used_ground_points, used_points_px)| {
                    (
                        circle,
                        context
                            .camera_matrix
                            .ground_to_pixel(circle.center)
                            .unwrap(),
                        used_points_px,
                    )
                })
                .collect_vec()
        });

        context
            .cycle_time
            .fill_if_subscribed(|| elapsed_time_after_all_processing);
        context.timings_for_steps_ms.fill_if_subscribed(|| {
            vec![
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
                    "edge_filtering_ms".to_string(),
                    (elapsed_time_after_lines - elapsed_time_after_edges).as_millis(),
                ),
                (
                    "circle_us".to_string(),
                    (elapsed_time_after_circles - elapsed_time_after_lines).as_micros(),
                ),
                (
                    "line filtering_ms".to_string(),
                    (elapsed_time_after_all_processing - elapsed_time_after_circles).as_millis(),
                ),
                (
                    "elapsed_time_after_all_processing_ms".to_string(),
                    (elapsed_time_after_all_processing).as_millis(),
                ),
            ]
        });

        // Set this as late as possible, to execute the next rount at least after the configured delay (checked at the beginning)
        self.last_processed_instance = Instant::now();

        Ok(MainOutputs {
            detected_calibration_circles: calibration_circles.into(),
        })
    }
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
    let y_exclusion_threshold: u32 = if let Some(threshold) = upper_points_exclusion_threshold_y {
        threshold
    } else {
        0
    };
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
    circle_centre: Point2<Ground>,
    circle_points: &[Point2<Ground>],
    circumference_percentage: f32,
) -> bool {
    let slices = 100;
    // Locations 0 to 100, also could be angle in degree or whatever
    // atan() -> [-PI/2, PI/2]
    let angle_to_slice_indice_factor = PI * 2.0 / (slices as f32);

    let present_slice_count = circle_points
        .into_iter()
        .map(|point| {
            let angle = (circle_centre.y() - point.y()).atan2((circle_centre.x() - point.x()));

            (angle / angle_to_slice_indice_factor).ceil() as i32
        })
        .unique()
        .count();

    let current_percentage = present_slice_count as f32 / slices as f32;

    current_percentage >= circumference_percentage.clamp(0.0, 1.0)
}

fn detect_circles(
    edge_points: &[Point2<Pixel>],
    camera_matrix: &CameraMatrix,
    maximum_number_of_circles: usize,
    ransac_iterations: usize,
    ransac_circle_inlier_threshold: f32,
    circumference_percentage: f32,
    target_circle_radius: f32,
) -> Vec<(Circle<Ground>, Vec<Point2<Ground>>, Vec<Point2<Pixel>>)> {
    let centre_distance_penalty_threshold = 10.0; // field length

    let edge_points_in_ground = edge_points
        .iter()
        .filter_map(|pixel_coordinates| {
            let point = camera_matrix.pixel_to_ground(*pixel_coordinates);
            point.ok().and_then(|point| {
                if distance(point, Point2::origin()) <= centre_distance_penalty_threshold {
                    Some(point)
                } else {
                    None
                }
            })
        })
        .collect_vec();

    let mut ransac = RansacCircleWithRadius::new(
        target_circle_radius,
        ransac_circle_inlier_threshold,
        edge_points_in_ground,
    );

    let results = (0..maximum_number_of_circles)
        .filter_map(
            |_| -> Option<(Circle<Ground>, Vec<Point2<Ground>>, Vec<Point2<Pixel>>)> {
                let result = ransac.next_candidate(ransac_iterations);
                result.output.and_then(|circle| {
                    let center_is_valid =
                        camera_matrix
                            .ground_to_pixel(circle.centre)
                            .is_ok_and(|center| {
                                center.y() <= camera_matrix.image_size.y()
                                    && circle_circumference_percentage_filter(
                                        circle.centre,
                                        &result.used_points,
                                        circumference_percentage,
                                    )
                            });

                    if center_is_valid {
                        let used_points_px = result
                            .used_points
                            .iter()
                            .map(|point| {
                                camera_matrix
                                    .ground_to_pixel(*point)
                                    .expect("pixel -> ground -> pixel: last failed")
                            })
                            .collect_vec();
                        Some((circle.into(), result.used_points, used_points_px))
                    } else {
                        None
                    }
                })
            },
        )
        .collect_vec();

    results
}
