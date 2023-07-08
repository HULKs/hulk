use std::{
    error::Error,
    time::{Duration, Instant},
};

use crate::image_ops::{generate_luminance_image, gray_image_to_hulks_grayscale_image};
use color_eyre::{eyre::eyre, Result};
use context_attribute::context;
use coordinate_systems::{Ground, Pixel};
use fast_image_resize::FilterType;
use framework::{deserialize_not_implemented, AdditionalOutput, MainOutput};
use geometry::circle::Circle;
use image::{GrayImage, Luma, RgbImage};
use imageproc::{edges::canny, filter::gaussian_blur_f32, map::map_colors};
use itertools::Itertools;
use linear_algebra::{distance, point, Point2};
use projection::{camera_matrix::CameraMatrix, Projection};
use rand::SeedableRng;
use rand_chacha::ChaChaRng;
use ransac::circles::circle_ransac::RansacCircleWithRadius;
use serde::{Deserialize, Serialize};
use types::{
    camera_position::CameraPosition, grayscale_image::GrayscaleImage, ycbcr422_image::YCbCr422Image,
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
    pub camera_position:
        Parameter<CameraPosition, "image_receiver.$cycler_instance.camera_position">,
    pub enable: Parameter<bool, "calibration_line_detection.$cycler_instance.enable">,
    pub canny_low_threshold: Parameter<f32, "calibration_line_detection.canny_low_threshold">,
    pub canny_high_threshold: Parameter<f32, "calibration_line_detection.canny_high_threshold">,
    pub gaussian_sigma: Parameter<f32, "calibration_line_detection.gaussian_sigma">,
    pub maximum_number_of_circles:
        Parameter<usize, "calibration_line_detection.maximum_number_of_circles">,
    pub ransac_iterations: Parameter<usize, "calibration_line_detection.ransac_iterations">,
    pub ransac_min_inlier_ratio:
        Parameter<f32, "calibration_line_detection.ransac_min_inlier_ratio">,
    pub ransac_maximum_gap: Parameter<f32, "calibration_line_detection.ransac_maximum_gap">,
    pub use_clustering_ransac: Parameter<bool, "calibration_line_detection.use_clustering_ransac">,
    pub debug_image_resized_width:
        Parameter<u32, "calibration_line_detection.debug_image_resized_width">,
    pub run_next_cycle_after_ms:
        Parameter<u64, "calibration_line_detection.run_next_cycle_after_ms">,
    // Heavier calculation due to rgb conversion
    pub skip_rgb_based_difference_image:
        Parameter<bool, "calibration_line_detection.skip_rgb_based_difference_image">,

    // TODO activate this once calibration controller can emit this value
    // pub camera_position_of_calibration_lines_request:
    //     RequiredInput<Option<CameraPosition>, "requested_calibration_lines?">,
    pub image: Input<YCbCr422Image, "image">,
    pub camera_matrix: RequiredInput<Option<CameraMatrix>, "camera_matrix?">,
    pub difference_image:
        AdditionalOutput<GrayscaleImage, "calibration_line_detection.difference_image">,
    pub blurred_image: AdditionalOutput<GrayscaleImage, "calibration_line_detection.blurred_image">,
    pub edges_image: AdditionalOutput<GrayscaleImage, "calibration_line_detection.edges_image">,
    pub unfiltered_circle:
        AdditionalOutput<Option<Circle<Ground>>, "calibration_line_detection.unfiltered_circles">,
    pub timings_for_steps_ms:
        AdditionalOutput<Vec<(String, u128)>, "calibration_line_detection.timings_for_steps">,
    pub cycle_time: AdditionalOutput<Duration, "calibration_line_detection.cycle_time">,
    pub circle_used_points:
        AdditionalOutput<Vec<Point2<Pixel>>, "calibration_line_detection.circle_used_points">,
}

#[context]
#[derive(Default)]
pub struct MainOutputs {
    pub detected_calibration_circles: MainOutput<Option<Circle<Ground>>>,
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
                detected_calibration_circles: None.into(),
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

        let circle_and_used_points = detect_circle(
            filtered_points,
            context.camera_matrix,
            *context.maximum_number_of_circles,
            *context.ransac_iterations,
            *context.ransac_min_inlier_ratio,
        );

        let elapsed_time_after_circles = processing_start.elapsed();

        let calibration_circles = circle_and_used_points.clone().map(|(circle, _, _)| circle);

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

        let (circle_option, used_points_px) =
            if let Some((circle, _used_points_gnd, used_points_px)) = circle_and_used_points {
                (Some(circle), used_points_px)
            } else {
                (None, vec![])
            };
        context
            .unfiltered_circle
            .fill_if_subscribed(|| circle_option);
        context
            .circle_used_points
            .fill_if_subscribed(|| used_points_px);

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

fn detect_circle(
    edge_points: Vec<Point2<Pixel>>,
    camera_matrix: &CameraMatrix,
    maximum_number_of_retries: usize,
    ransac_iterations: usize,
    ransac_min_inlier_ratio: f32,
) -> Option<(Circle<Ground>, Vec<Point2<Ground>>, Vec<Point2<Pixel>>)> {
    let radius_variance = 0.05; // 10cm
    let _centre_distance_penalty_threshold = 10.0; // field length

    let edge_points_in_ground = edge_points
        .iter()
        .filter_map(|pixel_coordinates| {
            let point = camera_matrix.pixel_to_ground(*pixel_coordinates);
            point.ok().and_then(|point| {
                if distance(point, point![0.0, 0.0]) <= _centre_distance_penalty_threshold {
                    Some(point)
                } else {
                    None
                }
            })
        })
        .collect_vec();

    let total_points = edge_points_in_ground.len();
    let mut ransac = RansacCircleWithRadius::new(0.75, radius_variance, edge_points_in_ground);

    for _ in 0..maximum_number_of_retries {
        let result = ransac.next_candidate(ransac_iterations);
        if let Some(circle) = result.output {
            let used_points_px = result
                .used_points
                .iter()
                .map(|point| camera_matrix.ground_to_pixel(*point)
                .expect("this transformation *must* succeed as the ground point comes from an existing pixel")).collect_vec();

            if (used_points_px.len() as f32) / (total_points as f32) >= ransac_min_inlier_ratio {
                return Some((circle.into(), result.used_points, used_points_px));
            }
        }
    }
    None
}
