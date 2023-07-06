use std::{
    num::NonZeroU32,
    time::{Duration, Instant},
};

use crate::ransac::ClusteringRansac;
use calibration::lines::GoalBoxCalibrationLines;
use color_eyre::Result;
use context_attribute::context;
use fast_image_resize::{DynamicImageView, FilterType, ImageView, ResizeAlg, Resizer};
use framework::{AdditionalOutput, MainOutput};
use image::{GrayImage, Luma, RgbImage};
use imageproc::{edges::canny, filter::gaussian_blur_f32, map::map_colors};
use lstsq::lstsq;
use nalgebra::{distance, point, DMatrix, DVector};
use types::{
    grayscale_image::GrayscaleImage, ycbcr422_image::YCbCr422Image, CameraPosition, Line, Line2,
};

pub struct CalibrationLineDetection {
    last_processed_instance: Instant,
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
    pub maximum_number_of_lines:
        Parameter<usize, "calibration_line_detection.maximum_number_of_lines">,
    pub ransac_iterations: Parameter<usize, "calibration_line_detection.ransac_iterations">,
    pub ransac_maximum_distance:
        Parameter<f32, "calibration_line_detection.ransac_maximum_distance">,
    pub ransac_maximum_gap: Parameter<f32, "calibration_line_detection.ransac_maximum_gap">,
    pub resized_width: Parameter<u32, "calibration_line_detection.resized_width">,
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
    pub difference_image:
        AdditionalOutput<GrayscaleImage, "calibration_line_detection.difference_image">,
    pub blurred_image: AdditionalOutput<GrayscaleImage, "calibration_line_detection.blurred_image">,
    pub edges_image: AdditionalOutput<GrayscaleImage, "calibration_line_detection.edges_image">,
    pub unfiltered_lines:
        AdditionalOutput<Option<Vec<Line2>>, "calibration_line_detection.unfiltered_lines">,
    pub timings_for_steps:
        AdditionalOutput<Vec<(String, Duration)>, "calibration_line_detection.timings_for_steps">,
    pub cycle_time: AdditionalOutput<Duration, "calibration_line_detection.cycle_time">,
}

#[context]
#[derive(Default)]
pub struct MainOutputs {
    pub detected_calibration_lines: MainOutput<Option<GoalBoxCalibrationLines>>,
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
                detected_calibration_lines: None.into(),
            });
        }

        let resized_image_size = {
            let aspect_ratio = context.image.height() as f32 / context.image.width() as f32;
            let expected_width = *context.resized_width;
            (
                expected_width,
                (expected_width as f32 * aspect_ratio) as u32,
            )
        };

        let debug_image_size = {
            let expected_width = *context.debug_image_resized_width;
            if expected_width >= resized_image_size.0 {
                None
            } else {
                let aspect_ratio = resized_image_size.1 as f32 / resized_image_size.0 as f32;

                Some((
                    expected_width,
                    (expected_width as f32 * aspect_ratio) as u32,
                ))
            }
        };

        let processing_start = Instant::now();
        let difference = {
            if *context.skip_rgb_based_difference_image {
                generate_luminance_image(context.image, resized_image_size)
                    .expect("Generating luma image failed")
            } else {
                let rgb = RgbImage::from(context.image);

                let difference = rgb_image_to_difference(&rgb);

                let resized = gray_image_resize(&difference, resized_image_size, None);
                GrayImage::from_vec(
                    resized.width().get(),
                    resized.height().get(),
                    resized.into_vec(),
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

        // Disabled this for now
        // let lines = detect_lines(
        //     &edges,
        //     *context.maximum_number_of_lines,
        //     *context.ransac_iterations,
        //     *context.ransac_maximum_distance,
        //     *context.ransac_maximum_gap,
        // );
        let lines: Option<Vec<Line2>> = None;
        let elapsed_time_after_lines = processing_start.elapsed();

        let calibration_lines = lines
            .as_ref()
            .and_then(|lines| filter_and_extract_calibration_lines(lines, &blurred));

        let elapsed_time_after_all_processing = processing_start.elapsed();

        context
            .cycle_time
            .fill_if_subscribed(|| elapsed_time_after_all_processing);

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

        context.unfiltered_lines.fill_if_subscribed(|| lines);
        context.timings_for_steps.fill_if_subscribed(|| {
            vec![
                ("difference".to_string(), elapsed_time_after_difference),
                (
                    "blurred".to_string(),
                    elapsed_time_after_blurred - elapsed_time_after_difference,
                ),
                (
                    "edges".to_string(),
                    elapsed_time_after_edges - elapsed_time_after_blurred,
                ),
                (
                    "lines".to_string(),
                    elapsed_time_after_lines - elapsed_time_after_edges,
                ),
                (
                    "line filtering".to_string(),
                    elapsed_time_after_all_processing - elapsed_time_after_lines,
                ),
            ]
        });

        // Set this as late as possible, to execute the next rount at least after the configured delay (checked at the beginning)
        self.last_processed_instance = Instant::now();

        Ok(MainOutputs {
            detected_calibration_lines: calibration_lines.into(),
        })
    }
}

#[inline]
fn gray_image_resize(
    image: &GrayImage,
    // new_image_view: &mut DynamicImageViewMut,
    new_size: (u32, u32),
    filter: Option<FilterType>,
) -> fast_image_resize::Image<'_> {
    let image_view = ImageView::from_buffer(
        NonZeroU32::new(image.width()).unwrap(),
        NonZeroU32::new(image.height()).unwrap(),
        &image.as_raw(),
    )
    .expect("ImageView creation failed!");
    let new_width = NonZeroU32::new(new_size.0).unwrap();
    let new_height = NonZeroU32::new(new_size.1).unwrap();
    let mut new_image =
        fast_image_resize::Image::new(new_width, new_height, image_view.pixel_type());
    let mut resizer = Resizer::new(ResizeAlg::Convolution(
        filter.unwrap_or_else(|| FilterType::Hamming),
    ));
    let mut new_image_view = new_image.view_mut();

    resizer
        .resize(&DynamicImageView::U8(image_view), &mut new_image_view)
        .unwrap();
    new_image
}

fn gray_image_to_hulks_grayscale_image(
    image: &GrayImage,
    new_size: Option<(u32, u32)>,
    filter: Option<FilterType>,
) -> GrayscaleImage {
    if let Some(new_size) = new_size {
        let resized = gray_image_resize(image, new_size, filter);
        GrayscaleImage::from_vec(
            resized.width().get(),
            resized.height().get(),
            resized.into_vec(),
        )
    } else {
        GrayscaleImage::from_vec(image.width(), image.height(), image.as_raw().clone())
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

fn generate_luminance_image(image: &YCbCr422Image, new_size: (u32, u32)) -> Option<GrayImage> {
    let grayscale_buffer: Vec<_> = image
        .buffer()
        .iter()
        .flat_map(|pixel| [pixel.y1, pixel.y2])
        .collect();
    let y_image = ImageView::from_buffer(
        NonZeroU32::new(image.width()).unwrap(),
        NonZeroU32::new(image.height()).unwrap(),
        &grayscale_buffer,
    );
    if let Ok(y_image) = y_image {
        let new_width = NonZeroU32::new(new_size.0).unwrap();
        let new_height = NonZeroU32::new(new_size.1).unwrap();
        let mut new_image =
            fast_image_resize::Image::new(new_width, new_height, y_image.pixel_type());
        let mut resizer = Resizer::new(ResizeAlg::Convolution(FilterType::Hamming));
        resizer
            .resize(&DynamicImageView::U8(y_image), &mut new_image.view_mut())
            .unwrap();
        GrayImage::from_vec(new_width.get(), new_height.get(), new_image.into_vec())
    } else {
        None
    }
}

fn detect_lines(
    edges: &GrayImage,
    maximum_number_of_lines: usize,
    ransac_iterations: usize,
    ransac_maximum_distance: f32,
    ransac_maximum_gap: f32,
) -> Option<Vec<Line2>> {
    let edge_points = edges
        .enumerate_pixels()
        .filter_map(|(x, y, color)| {
            if color[0] > 127 {
                Some(point![x as f32, y as f32])
            } else {
                None
            }
        })
        .collect();
    let mut ransac = ClusteringRansac::new(edge_points);
    let mut lines = vec![];
    for _ in 0..maximum_number_of_lines {
        let used_points = ransac.next_line_cluster(
            ransac_iterations,
            ransac_maximum_distance,
            ransac_maximum_gap,
        );
        if used_points.is_empty() {
            break;
        }
        let start_x = used_points
            .iter()
            .min_by(|left, right| left[0].total_cmp(&right[0]))
            .unwrap()[0];
        let end_x = used_points
            .iter()
            .max_by(|left, right| left[0].total_cmp(&right[0]))
            .unwrap()[0];
        let (mut x, y) = used_points
            .into_iter()
            .fold((vec![], vec![]), |(mut x, mut y), point| {
                x.push(point.x);
                y.push(point.y);
                (x, y)
            });
        x.resize(x.len() * 2, 1.0);
        let x = DMatrix::from_vec(x.len() / 2, 2, x);
        let y = DVector::from_vec(y);
        let result = lstsq(&x, &y, 1e-7).ok()?;
        let start = point![start_x, (start_x * result.solution[0] + result.solution[1])];
        let end = point![end_x, (end_x * result.solution[0] + result.solution[1])];
        lines.push(Line(start, end));
    }
    Some(lines)
}

fn filter_and_extract_calibration_lines(
    lines: &[Line2],
    blurred: &GrayImage,
) -> Option<GoalBoxCalibrationLines> {
    if lines.len() < 4 {
        return None;
    }
    let lines = lines.to_vec();
    let mut lines_with_edge_positions: Vec<_> = lines
        .into_iter()
        .map(|line| {
            let line_edge_position = line_edge_position(line, blurred);
            if line.0.y > line.1.y {
                (Line(line.1, line.0), line_edge_position)
            } else {
                (line, line_edge_position)
            }
        })
        .collect();
    lines_with_edge_positions.sort_by(
        |(left_line, _left_line_edge_position), (right_line, _right_line_edge_position)| {
            left_line.1.y.total_cmp(&right_line.1.y)
        },
    );

    let mut lowest_four_lines_with_edge_positions =
        lines_with_edge_positions.split_off(lines_with_edge_positions.len() - 4);
    let lowest_line = lowest_four_lines_with_edge_positions.last().unwrap().0;
    let line_end_points_too_far_apart = lowest_four_lines_with_edge_positions
        .iter()
        .any(|(line, _edge_position)| distance(&line.1, &lowest_line.1) > 70.0);
    if line_end_points_too_far_apart {
        return None;
    }

    lowest_four_lines_with_edge_positions.sort_by(
        |(left_line, _left_edge_position), (right_line, _right_edge_position)| {
            left_line.length().total_cmp(&right_line.length())
        },
    );
    let _corner_to_border = lowest_four_lines_with_edge_positions.split_off(2);
    let _corner_to_line_end = lowest_four_lines_with_edge_positions;
    let _second_lowest_line = lines_with_edge_positions.pop()?;
    let _lowest_point = if lowest_line.0.y <= lowest_line.1.y {
        lowest_line.0
    } else {
        lowest_line.1
    };

    todo!();
}

enum LineEdgePosition {
    Upper,
    Lower,
}

fn line_edge_position(line: Line2, blurred: &GrayImage) -> LineEdgePosition {
    let (x_start, x_end) = if line.0.x <= line.1.x {
        (line.0.x as u32, line.1.x as u32)
    } else {
        (line.1.x as u32, line.0.x as u32)
    };
    let slope = line.slope();
    let y_axis_intercept = line.y_axis_intercept();
    let sum: i32 = (x_start..x_end)
        .map(|x| {
            let y = (slope * x as f32 + y_axis_intercept) as u32;
            let upper = blurred.get_pixel_checked(x, y - 20);
            let lower = blurred.get_pixel_checked(x, y + 20);
            match (upper, lower) {
                (Some(upper), Some(lower)) => lower[0] as i32 - upper[0] as i32,
                _ => 0,
            }
        })
        .sum();
    if sum >= 0 {
        LineEdgePosition::Upper
    } else {
        LineEdgePosition::Lower
    }
}
