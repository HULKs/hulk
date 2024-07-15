use std::{
    ops::Range,
    time::{Duration, Instant},
};

use color_eyre::Result;
use projection::{camera_matrix::CameraMatrix, horizon::Horizon, Projection};
use serde::{Deserialize, Serialize};

use context_attribute::context;
use coordinate_systems::{Field, Ground, Pixel};
use framework::{AdditionalOutput, MainOutput};
use linear_algebra::{point, vector, Isometry2, Point2, Transform, Vector2};
use types::{
    color::{Intensity, Rgb, YCbCr444},
    field_color::FieldColor,
    image_segments::{Direction, EdgeType, ImageSegments, ScanGrid, ScanLine, Segment},
    interpolated::Interpolated,
    limb::{is_above_limbs, Limb, ProjectedLimbs},
    parameters::{EdgeDetectionSourceParameters, MedianModeParameters},
    ycbcr422_image::YCbCr422Image,
};

#[derive(Deserialize, Serialize)]
pub struct ImageSegmenter {
    ground_to_field_of_home_after_coin_toss_before_second_half: Isometry2<Ground, Field>,
}

#[context]
pub struct CreationContext {}

#[context]
pub struct CycleContext {
    image_segmenter_cycle_time: AdditionalOutput<Duration, "image_segmenter_cycle_time">,

    image: Input<YCbCr422Image, "image">,

    camera_matrix: Input<Option<CameraMatrix>, "camera_matrix?">,
    ground_to_field_of_home_after_coin_toss_before_second_half: Input<
        Option<Isometry2<Ground, Field>>,
        "Control",
        "ground_to_field_of_home_after_coin_toss_before_second_half?",
    >,
    field_color: Input<FieldColor, "field_color">,
    projected_limbs: Input<Option<ProjectedLimbs>, "projected_limbs?">,

    horizontal_stride: Parameter<usize, "image_segmenter.$cycler_instance.horizontal_stride">,
    horizontal_edge_detection_source: Parameter<
        EdgeDetectionSourceParameters,
        "image_segmenter.$cycler_instance.horizontal_edge_detection_source",
    >,
    horizontal_edge_threshold:
        Parameter<Interpolated, "image_segmenter.$cycler_instance.horizontal_edge_threshold">,
    horizontal_median_mode:
        Parameter<MedianModeParameters, "image_segmenter.$cycler_instance.horizontal_median_mode">,
    vertical_stride: Parameter<usize, "image_segmenter.$cycler_instance.vertical_stride">,
    vertical_edge_detection_source: Parameter<
        EdgeDetectionSourceParameters,
        "image_segmenter.$cycler_instance.vertical_edge_detection_source",
    >,
    vertical_edge_threshold:
        Parameter<Interpolated, "image_segmenter.$cycler_instance.vertical_edge_threshold">,
    vertical_median_mode:
        Parameter<MedianModeParameters, "image_segmenter.$cycler_instance.vertical_median_mode">,
}

#[context]
#[derive(Default)]
pub struct MainOutputs {
    pub image_segments: MainOutput<ImageSegments>,
}

impl ImageSegmenter {
    pub fn new(_context: CreationContext) -> Result<Self> {
        Ok(Self {
            ground_to_field_of_home_after_coin_toss_before_second_half: Transform::default(),
        })
    }

    pub fn cycle(&mut self, mut context: CycleContext) -> Result<MainOutputs> {
        if let Some(ground_to_field_of_home_after_coin_toss_before_second_half) =
            context.ground_to_field_of_home_after_coin_toss_before_second_half
        {
            self.ground_to_field_of_home_after_coin_toss_before_second_half =
                *ground_to_field_of_home_after_coin_toss_before_second_half;
        }

        let begin = Instant::now();
        let projected_limbs = context
            .projected_limbs
            .map_or(Default::default(), |projected_limbs| {
                projected_limbs.limbs.as_slice()
            });

        let horizon = context
            .camera_matrix
            .and_then(|camera_matrix| camera_matrix.horizon)
            .unwrap_or(Horizon::ABOVE_IMAGE);

        let vertical_stride_in_robot_coordinates = 0.02;

        let scan_grid = new_grid(
            context.image,
            context.camera_matrix,
            &horizon,
            context.field_color,
            *context.horizontal_stride,
            context
                .horizontal_edge_threshold
                .evaluate_at(self.ground_to_field_of_home_after_coin_toss_before_second_half)
                as i16,
            *context.horizontal_median_mode,
            *context.horizontal_edge_detection_source,
            *context.vertical_stride,
            vertical_stride_in_robot_coordinates,
            *context.vertical_edge_detection_source,
            context
                .vertical_edge_threshold
                .evaluate_at(self.ground_to_field_of_home_after_coin_toss_before_second_half)
                as i16,
            *context.vertical_median_mode,
            projected_limbs,
        );
        let end = Instant::now();
        context
            .image_segmenter_cycle_time
            .fill_if_subscribed(|| end - begin);
        Ok(MainOutputs {
            image_segments: ImageSegments { scan_grid }.into(),
        })
    }
}

fn padding_size(median_mode: MedianModeParameters) -> u32 {
    match median_mode {
        MedianModeParameters::Disabled => 0,
        MedianModeParameters::ThreePixels => 1,
        MedianModeParameters::FivePixels => 2,
    }
}

#[allow(clippy::too_many_arguments)]
fn new_grid(
    image: &YCbCr422Image,
    camera_matrix: Option<&CameraMatrix>,
    horizon: &Horizon,
    field_color: &FieldColor,
    horizontal_stride: usize,
    horizontal_edge_threshold: i16,
    horizontal_median_mode: MedianModeParameters,
    horizontal_edge_detection_source: EdgeDetectionSourceParameters,
    vertical_stride: usize,
    vertical_stride_in_robot_coordinates: f32,
    vertical_edge_detection_source: EdgeDetectionSourceParameters,
    vertical_edge_threshold: i16,
    vertical_median_mode: MedianModeParameters,
    projected_limbs: &[Limb],
) -> ScanGrid {
    let horizontal_padding_size = padding_size(horizontal_median_mode);
    let vertical_padding_size = padding_size(vertical_median_mode);

    let horizon_y_maximum = horizon
        .horizon_y_maximum()
        .clamp(0.0, image.height() as f32);
    let mut horizontal_scan_lines = vec![];
    // do not start at horizon because of numerically unstable math
    let mut y = horizon_y_maximum + 1.0 + horizontal_padding_size as f32;

    while y < (image.height() - horizontal_padding_size) as f32 {
        horizontal_scan_lines.push(new_horizontal_scan_line(
            image,
            field_color,
            y as u32,
            horizontal_stride,
            horizontal_edge_detection_source,
            horizontal_edge_threshold,
            horizontal_median_mode,
        ));

        y = next_y_from_current_y(
            image,
            camera_matrix,
            vertical_stride_in_robot_coordinates,
            y,
        )
        .unwrap_or(0.0)
        .max(y + 2.0);
    }

    ScanGrid {
        horizontal_scan_lines,
        vertical_scan_lines: (vertical_padding_size..image.width() - vertical_padding_size)
            .step_by(horizontal_stride)
            .map(|x| {
                let horizon_y = horizon.y_at_x(x as f32).clamp(0.0, image.height() as f32);
                new_vertical_scan_line(
                    image,
                    field_color,
                    x,
                    vertical_stride,
                    vertical_edge_detection_source,
                    vertical_edge_threshold,
                    vertical_median_mode,
                    horizon_y,
                    projected_limbs,
                )
            })
            .collect(),
    }
}

fn next_y_from_current_y(
    image: &YCbCr422Image,
    camera_matrix: Option<&CameraMatrix>,
    vertical_stride_in_robot_coordinates: f32,
    y: f32,
) -> Option<f32> {
    let camera_matrix = camera_matrix?;

    let center_point_at_y = point![(image.width() / 2) as f32, y];
    let center_point_in_robot_coordinates =
        camera_matrix.pixel_to_ground(center_point_at_y).ok()?;

    let x_in_robot_coordinates = center_point_in_robot_coordinates.x();
    let y_in_robot_coordinates = center_point_in_robot_coordinates.y();
    let next_x_in_robot_coordinates = x_in_robot_coordinates - vertical_stride_in_robot_coordinates;

    let next_center_point_in_robot_coordinates =
        point![next_x_in_robot_coordinates, y_in_robot_coordinates];
    let next_point_in_pixel_coordinates = camera_matrix
        .ground_to_pixel(next_center_point_in_robot_coordinates)
        .ok()?;

    Some(next_point_in_pixel_coordinates.y())
}

struct ScanLineState {
    previous_value: i16,
    previous_difference: i16,
    maximum_difference: i16,
    maximum_difference_position: u16,
    start_position: u16,
    start_edge_type: EdgeType,
}

impl ScanLineState {
    fn new(previous_value: i16, start_position: u16, start_edge_type: EdgeType) -> Self {
        Self {
            previous_value,
            previous_difference: Default::default(),
            maximum_difference: Default::default(),
            maximum_difference_position: Default::default(),
            start_position,
            start_edge_type,
        }
    }
}

fn median_of_three(values: [u8; 3]) -> u8 {
    let [first, second, third] = values;
    // TODO: replace with same approach as median_of_five()
    if first <= second {
        if second <= third {
            // first <= second <= third
            second
        } else if first <= third {
            // first <= third < second
            third
        } else {
            // third < first <= second
            first
        }
    } else if first <= third {
        // second < first <= third
        first
    } else if second <= third {
        // second <= third < first
        third
    } else {
        // third < second <= first
        second
    }
}

fn median_of_five(mut values: [u8; 5]) -> u8 {
    let (_, median, _) = values.select_nth_unstable(2);
    *median
}

fn new_horizontal_scan_line(
    image: &YCbCr422Image,
    field_color: &FieldColor,
    position: u32,
    stride: usize,
    edge_detection_source: EdgeDetectionSourceParameters,
    edge_threshold: i16,
    median_mode: MedianModeParameters,
) -> ScanLine {
    let start_x = 0;
    let end_x = image.width();

    let edge_detection_value = edge_detection_value_at(
        Direction::Horizontal,
        point!(start_x, position),
        image,
        edge_detection_source,
        median_mode,
    );
    let mut state = ScanLineState::new(edge_detection_value, start_x as u16, EdgeType::ImageBorder);

    let mut segments = Vec::with_capacity((end_x - start_x) as usize / stride);

    for x in (start_x..end_x).step_by(stride) {
        let edge_detection_value = edge_detection_value_at(
            Direction::Horizontal,
            point!(x, position),
            image,
            edge_detection_source,
            median_mode,
        );

        if let Some(segment) =
            detect_edge(&mut state, x as u16, edge_detection_value, edge_threshold)
        {
            segments.push(set_field_color_in_segment(
                set_color_in_horizontal_segment(segment, image, position),
                field_color,
            ));
        }
    }

    let last_segment = Segment {
        start: state.start_position,
        end: image.width() as u16,
        start_edge_type: state.start_edge_type,
        end_edge_type: EdgeType::ImageBorder,
        color: Default::default(),
        field_color: Intensity::Low,
    };
    segments.push(set_field_color_in_segment(
        set_color_in_horizontal_segment(last_segment, image, position),
        field_color,
    ));

    ScanLine {
        position: position as u16,
        segments,
    }
}

#[allow(clippy::too_many_arguments)]
fn new_vertical_scan_line(
    image: &YCbCr422Image,
    field_color: &FieldColor,
    position: u32,
    stride: usize,
    edge_detection_source: EdgeDetectionSourceParameters,
    edge_threshold: i16,
    median_mode: MedianModeParameters,
    horizon_y: f32,
    projected_limbs: &[Limb],
) -> ScanLine {
    let start_y = horizon_y as u32;
    let end_y = image.height();

    if start_y >= end_y {
        return ScanLine {
            position: position as u16,
            segments: Vec::new(),
        };
    }

    let edge_detection_value = edge_detection_value_at(
        Direction::Vertical,
        point!(position, start_y),
        image,
        edge_detection_source,
        median_mode,
    );
    let mut state = ScanLineState::new(edge_detection_value, start_y as u16, EdgeType::ImageBorder);

    let mut segments = Vec::with_capacity((end_y - start_y) as usize / stride);
    for y in (start_y..end_y).step_by(stride) {
        let edge_detection_value = edge_detection_value_at(
            Direction::Vertical,
            point!(position, y),
            image,
            edge_detection_source,
            median_mode,
        );

        if let Some(segment) =
            detect_edge(&mut state, y as u16, edge_detection_value, edge_threshold)
        {
            if segment_is_below_limbs(position as u16, &segment, projected_limbs) {
                fix_previous_edge_type(&mut segments);
                break;
            }
            segments.push(set_field_color_in_segment(
                set_color_in_vertical_segment(segment, image, position),
                field_color,
            ));
        }
    }

    let last_segment = Segment {
        start: state.start_position,
        end: image.height() as u16,
        start_edge_type: state.start_edge_type,
        end_edge_type: EdgeType::ImageBorder,
        color: Default::default(),
        field_color: Intensity::Low,
    };
    if !segment_is_below_limbs(position as u16, &last_segment, projected_limbs) {
        segments.push(set_field_color_in_segment(
            set_color_in_vertical_segment(last_segment, image, position),
            field_color,
        ));
    }

    ScanLine {
        position: position as u16,
        segments,
    }
}

fn edge_detection_value_at(
    direction: Direction,
    position: Point2<Pixel, u32>,
    image: &YCbCr422Image,
    edge_detection_source: EdgeDetectionSourceParameters,
    median_mode: MedianModeParameters,
) -> i16 {
    let offset: Vector2<Pixel, u32> = match direction {
        Direction::Horizontal => vector!(0, 1),
        Direction::Vertical => vector!(1, 0),
    };

    let pixel = pixel_to_edge_detection_value(image.at_point(position), edge_detection_source);

    (match median_mode {
        MedianModeParameters::Disabled => pixel,
        MedianModeParameters::ThreePixels => {
            let pixels = [
                image.at_point(position - offset),
                image.at_point(position),
                image.at_point(position + offset),
            ]
            .map(|pixel| pixel_to_edge_detection_value(pixel, edge_detection_source));
            median_of_three(pixels)
        }
        MedianModeParameters::FivePixels => {
            let pixels = [
                image.at_point(position - offset * 2),
                image.at_point(position - offset),
                image.at_point(position),
                image.at_point(position + offset),
                image.at_point(position + offset * 2),
            ]
            .map(|pixel| pixel_to_edge_detection_value(pixel, edge_detection_source));
            median_of_five(pixels)
        }
    } as i16)
}

fn pixel_to_edge_detection_value(
    pixel: YCbCr444,
    edge_detection_source: EdgeDetectionSourceParameters,
) -> u8 {
    match edge_detection_source {
        EdgeDetectionSourceParameters::Luminance => pixel.y,
        EdgeDetectionSourceParameters::GreenChromaticity => {
            let rgb = Rgb::from(pixel);
            (rgb.convert_to_rgchromaticity().green * 255.0) as u8
        }
    }
}

fn set_color_in_vertical_segment(mut segment: Segment, image: &YCbCr422Image, x: u32) -> Segment {
    let length = segment.length();
    let stride = match length {
        20.. => 4,   // results in 5.. or more sample pixels
        7..=19 => 2, // results in 4..=10 or more sample pixels
        1..=6 => 1,  // results in 1..=6 or more sample pixels
        0 => {
            segment.color = image.at(x, segment.start as u32);
            return segment;
        }
    };
    let mut pixels = sample_image_pixels_vertically(
        image,
        x,
        (segment.start as u32)..(segment.end as u32),
        stride,
    );
    let length = pixels.len();
    let (_, median_pixel, _) = pixels.select_nth_unstable_by_key(length / 2, |pixel| pixel.y);
    segment.color = *median_pixel;
    segment
}

fn set_field_color_in_segment(mut segment: Segment, field_color: &FieldColor) -> Segment {
    segment.field_color = field_color.get_intensity(segment.color);
    segment
}

fn sample_image_pixels_horizontally(
    image: &YCbCr422Image,
    x: Range<u32>,
    y: u32,
    stride: u32,
) -> Vec<YCbCr444> {
    let number_of_steps = ((x.end - x.start) + (stride - 1)) / stride; // divide rounding up

    x.step_by(stride as usize)
        .take(number_of_steps as usize)
        .map(|x| image.at(x, y))
        .collect()
}

fn sample_image_pixels_vertically(
    image: &YCbCr422Image,
    x: u32,
    y: Range<u32>,
    stride: u32,
) -> Vec<YCbCr444> {
    let skip = image.coordinates_to_buffer_index(x, y.start);
    let step = image.coordinates_to_buffer_index(x, y.start + stride) - skip;
    let number_of_steps = ((y.end - y.start) + (stride - 1)) / stride; // divide rounding up
    let is_left_pixel = x % 2 == 0;
    image
        .buffer()
        .iter()
        .skip(skip)
        .step_by(step)
        .take(number_of_steps as usize)
        .map(|pixel_422| YCbCr444 {
            y: if is_left_pixel {
                pixel_422.y1
            } else {
                pixel_422.y2
            },
            cb: pixel_422.cb,
            cr: pixel_422.cr,
        })
        .collect()
}

fn segment_is_below_limbs(
    scan_line_position: u16,
    segment: &Segment,
    projected_limbs: &[Limb],
) -> bool {
    !is_above_limbs(
        point![scan_line_position as f32, segment.end as f32],
        projected_limbs,
    )
}

fn fix_previous_edge_type(segments: &mut [Segment]) {
    if let Some(previous_segment) = segments.last_mut() {
        previous_segment.end_edge_type = EdgeType::LimbBorder;
    }
}

fn set_color_in_horizontal_segment(mut segment: Segment, image: &YCbCr422Image, y: u32) -> Segment {
    let length = segment.length();
    let stride = match length {
        20.. => 4,   // results in 5.. or more sample pixels
        7..=19 => 2, // results in 4..=10 or more sample pixels
        1..=6 => 1,  // results in 1..=6 or more sample pixels
        0 => {
            segment.color = image.at(segment.start as u32, y);
            return segment;
        }
    };
    let mut pixels = sample_image_pixels_horizontally(
        image,
        (segment.start as u32)..(segment.end as u32),
        y,
        stride,
    );
    let length = pixels.len();
    let (_, median_pixel, _) = pixels.select_nth_unstable_by_key(length / 2, |pixel| pixel.y);
    segment.color = *median_pixel;
    segment
}

fn detect_edge(
    state: &mut ScanLineState,
    position: u16,
    value: i16,
    edge_threshold: i16,
) -> Option<Segment> {
    let value_difference = value - state.previous_value;

    let differences_have_initial_values = state.maximum_difference == 0 && value_difference == 0;
    let new_difference_is_more_positive =
        state.maximum_difference >= 0 && value_difference >= state.maximum_difference;
    let new_difference_is_more_negative =
        state.maximum_difference <= 0 && value_difference <= state.maximum_difference;

    if value_difference.abs() >= edge_threshold
        && (differences_have_initial_values
            || new_difference_is_more_positive
            || new_difference_is_more_negative)
    {
        state.maximum_difference = value_difference;
        state.maximum_difference_position = position;
    }

    let found_rising_edge =
        state.previous_difference >= edge_threshold && value_difference < edge_threshold;
    let found_falling_edge =
        state.previous_difference <= -edge_threshold && value_difference > -edge_threshold;

    let segment = if found_rising_edge || found_falling_edge {
        let end_edge_type = if found_rising_edge {
            EdgeType::Rising
        } else {
            EdgeType::Falling
        };
        let segment = Segment {
            start: state.start_position,
            end: state.maximum_difference_position,
            start_edge_type: state.start_edge_type,
            end_edge_type,
            color: Default::default(),
            field_color: Intensity::Low,
        };
        state.maximum_difference = 0;
        state.start_position = state.maximum_difference_position;
        state.start_edge_type = end_edge_type;

        Some(segment)
    } else {
        None
    };

    state.previous_value = value;
    state.previous_difference = value_difference;

    segment
}

#[cfg(test)]
mod tests {
    use itertools::iproduct;
    use types::{color::YCbCr422, field_color::FieldColorFunction};

    use super::*;

    #[test]
    fn maximum_with_sign_switch() {
        let image = YCbCr422Image::load_from_444_png(
            "../../tests/data/white_wall_with_a_little_desk_in_front.png",
        )
        .unwrap();
        let field_color = FieldColor {
            red_chromaticity_threshold: 0.37,
            blue_chromaticity_threshold: 0.38,
            green_chromaticity_threshold: 0.43,
            green_luminance_threshold: 255.0,
            luminance_threshold: 25.0,
            function: FieldColorFunction::GreenChromaticity,
            hue_low_threshold: 0.0,
            hue_high_threshold: 0.0,
            saturation_low_threshold: 0.0,
            saturation_high_threshold: 0.0,
        };
        let vertical_stride = 2;
        let vertical_edge_threshold = 16;
        let vertical_median_mode = MedianModeParameters::Disabled;
        let vertical_edge_detection_source = EdgeDetectionSourceParameters::Luminance;
        let horizon_y_minimum = 0.0;
        let scan_line = new_vertical_scan_line(
            &image,
            &field_color,
            12,
            vertical_stride,
            vertical_edge_detection_source,
            vertical_edge_threshold,
            vertical_median_mode,
            horizon_y_minimum,
            &[],
        );
        assert_eq!(scan_line.position, 12);
        assert!(scan_line.segments.len() >= 3);
        assert_eq!(scan_line.segments[0].start, 0);
        assert!(scan_line.segments[0].length() >= 255);
        assert!(scan_line.segments[0].length() <= 270);
        assert!(scan_line.segments[1].length() >= 45);
        assert!(scan_line.segments[1].length() <= 55);
    }

    #[test]
    fn image_with_one_vertical_segment_without_median() {
        let image = YCbCr422Image::zero(6, 3);
        let field_color = FieldColor {
            red_chromaticity_threshold: 0.37,
            blue_chromaticity_threshold: 0.38,
            green_chromaticity_threshold: 0.43,
            green_luminance_threshold: 255.0,
            luminance_threshold: 25.0,
            function: FieldColorFunction::GreenChromaticity,
            hue_low_threshold: 0.0,
            hue_high_threshold: 0.0,
            saturation_low_threshold: 0.0,
            saturation_high_threshold: 0.0,
        };
        let scan_line = new_vertical_scan_line(
            &image,
            &field_color,
            0,
            2,
            EdgeDetectionSourceParameters::Luminance,
            1,
            MedianModeParameters::Disabled,
            0.0,
            &[],
        );
        assert_eq!(scan_line.position, 0);
        assert_eq!(scan_line.segments.len(), 1);
        assert_eq!(scan_line.segments[0].start, 0);
        assert_eq!(scan_line.segments[0].end, 3);
    }

    #[test]
    fn image_with_one_vertical_segment_with_median() {
        let image = YCbCr422Image::zero(6, 3);
        let field_color = FieldColor {
            red_chromaticity_threshold: 0.37,
            blue_chromaticity_threshold: 0.38,
            green_chromaticity_threshold: 0.43,
            green_luminance_threshold: 255.0,
            luminance_threshold: 25.0,
            function: FieldColorFunction::GreenChromaticity,
            hue_low_threshold: 0.0,
            hue_high_threshold: 0.0,
            saturation_low_threshold: 0.0,
            saturation_high_threshold: 0.0,
        };
        let scan_line = new_vertical_scan_line(
            &image,
            &field_color,
            1,
            2,
            EdgeDetectionSourceParameters::Luminance,
            1,
            MedianModeParameters::ThreePixels,
            0.0,
            &[],
        );
        assert_eq!(scan_line.position, 1);
        assert_eq!(scan_line.segments.len(), 1);
        assert_eq!(scan_line.segments[0].start, 0);
        assert_eq!(scan_line.segments[0].end, 3);
    }

    #[test]
    fn image_vertical_color_three_pixels() {
        let image = YCbCr422Image::from_ycbcr_buffer(
            1,
            3,
            vec![
                // only evaluating every second 422 pixel
                YCbCr422::new(0, 10, 10, 10),
                YCbCr422::new(0, 15, 14, 13),
                YCbCr422::new(0, 10, 10, 10),
            ],
        );
        let field_color = FieldColor {
            red_chromaticity_threshold: 0.37,
            blue_chromaticity_threshold: 0.38,
            green_chromaticity_threshold: 0.43,
            green_luminance_threshold: 255.0,
            luminance_threshold: 25.0,
            function: FieldColorFunction::GreenChromaticity,
            hue_low_threshold: 0.0,
            hue_high_threshold: 0.0,
            saturation_low_threshold: 0.0,
            saturation_high_threshold: 0.0,
        };

        let scan_line = new_vertical_scan_line(
            &image,
            &field_color,
            0,
            2,
            EdgeDetectionSourceParameters::Luminance,
            1,
            MedianModeParameters::Disabled,
            0.0,
            &[],
        );
        assert_eq!(scan_line.position, 0);
        assert_eq!(scan_line.segments.len(), 1);
        assert_eq!(scan_line.segments[0].color.y, 0);
        assert_eq!(scan_line.segments[0].color.cb, 15);
        assert_eq!(scan_line.segments[0].color.cr, 13);
    }

    #[test]
    fn image_vertical_color_twelve_pixels() {
        let image = YCbCr422Image::from_ycbcr_buffer(
            1,
            12,
            vec![
                YCbCr422::new(0, 10, 10, 10),
                YCbCr422::new(0, 10, 10, 10),
                YCbCr422::new(0, 10, 10, 10),
                YCbCr422::new(0, 1, 1, 1),
                YCbCr422::new(0, 10, 10, 10),
                YCbCr422::new(0, 10, 10, 10),
                YCbCr422::new(0, 1, 1, 1),
                YCbCr422::new(0, 10, 10, 10),
                YCbCr422::new(0, 10, 10, 10),
                YCbCr422::new(0, 2, 2, 2),
                YCbCr422::new(0, 10, 10, 10),
                YCbCr422::new(0, 10, 10, 10),
            ],
        );
        let field_color = FieldColor {
            red_chromaticity_threshold: 0.37,
            blue_chromaticity_threshold: 0.38,
            green_chromaticity_threshold: 0.43,
            green_luminance_threshold: 255.0,
            luminance_threshold: 25.0,
            function: FieldColorFunction::GreenChromaticity,
            hue_low_threshold: 0.0,
            hue_high_threshold: 0.0,
            saturation_low_threshold: 0.0,
            saturation_high_threshold: 0.0,
        };

        let scan_line = new_vertical_scan_line(
            &image,
            &field_color,
            0,
            2,
            EdgeDetectionSourceParameters::Luminance,
            1,
            MedianModeParameters::Disabled,
            0.0,
            &[],
        );
        assert_eq!(scan_line.position, 0);
        assert_eq!(scan_line.segments.len(), 1);
        assert_eq!(scan_line.segments[0].color.y, 0);
        assert_eq!(scan_line.segments[0].color.cb, 1);
        assert_eq!(scan_line.segments[0].color.cr, 1);
    }

    #[test]
    fn image_with_three_vertical_increasing_segments_without_median() {
        let image = YCbCr422Image::from_ycbcr_buffer(
            1,
            12,
            vec![
                // only evaluating every secondth pixel
                YCbCr422::new(0, 0, 0, 0),
                YCbCr422::new(0, 0, 0, 0), // skipped
                // segment boundary will be here
                YCbCr422::new(1, 0, 0, 0),
                YCbCr422::new(1, 0, 0, 0), // skipped
                YCbCr422::new(1, 0, 0, 0),
                YCbCr422::new(1, 0, 0, 0), // skipped
                // segment boundary will be here
                YCbCr422::new(2, 0, 0, 0),
                YCbCr422::new(2, 0, 0, 0), // skipped
                YCbCr422::new(2, 0, 0, 0),
                YCbCr422::new(2, 0, 0, 0), // skipped
                YCbCr422::new(3, 0, 0, 0),
                YCbCr422::new(3, 0, 0, 0), // skipped

                                           // segment boundary will be here
            ],
        );
        let field_color = FieldColor {
            red_chromaticity_threshold: 0.37,
            blue_chromaticity_threshold: 0.38,
            green_chromaticity_threshold: 0.43,
            green_luminance_threshold: 255.0,
            luminance_threshold: 25.0,
            function: FieldColorFunction::GreenChromaticity,
            hue_low_threshold: 0.0,
            hue_high_threshold: 0.0,
            saturation_low_threshold: 0.0,
            saturation_high_threshold: 0.0,
        };

        // y  diff  prev_diff  prev_diff >= thres  diff < thres  prev_diff >= thres && diff < thres
        // 0  0     0          0                   1             0
        // 1  1     0          0                   0             0
        // 1  0     1          1                   1             1 -> end segment at position 2
        // 2  1     0          0                   0             0
        // 2  0     1          1                   1             1 -> end segment at position 6
        // 3  1     0          0                   0             0
        // -> end segment at position 12

        let scan_line = new_vertical_scan_line(
            &image,
            &field_color,
            0,
            2,
            EdgeDetectionSourceParameters::Luminance,
            1,
            MedianModeParameters::Disabled,
            0.0,
            &[],
        );
        assert_eq!(scan_line.position, 0);
        assert_eq!(scan_line.segments.len(), 3);
        assert_eq!(scan_line.segments[0].start, 0);
        assert_eq!(scan_line.segments[0].end, 2);
        assert_eq!(scan_line.segments[1].start, 2);
        assert_eq!(scan_line.segments[1].end, 6);
        assert_eq!(scan_line.segments[2].start, 6);
        assert_eq!(scan_line.segments[2].end, 12);
    }

    #[test]
    fn image_with_three_vertical_increasing_segments_with_median() {
        let row = |y| {
            [
                YCbCr422::new(y, 0, 0, 0),
                YCbCr422::new(y, 0, 0, 0),
                YCbCr422::new(y, 0, 0, 0),
            ]
        };
        let image = YCbCr422Image::from_ycbcr_buffer(
            3,
            12,
            [
                // only evaluating every second pixel
                row(0),
                row(0), // skipped
                row(1),
                // segment boundary will be here
                row(1), // skipped
                row(1),
                row(1), // skipped
                row(2),
                // segment boundary will be here
                row(2), // skipped
                row(2),
                row(2), // skipped
                row(3),
                row(3), // skipped
                        // segment boundary will be here
            ]
            .into_iter()
            .flatten()
            .collect(),
        );
        let field_color = FieldColor {
            red_chromaticity_threshold: 0.37,
            blue_chromaticity_threshold: 0.38,
            green_chromaticity_threshold: 0.43,
            green_luminance_threshold: 255.0,
            luminance_threshold: 25.0,
            function: FieldColorFunction::GreenChromaticity,
            hue_low_threshold: 0.0,
            hue_high_threshold: 0.0,
            saturation_low_threshold: 0.0,
            saturation_high_threshold: 0.0,
        };

        // y  y_median  diff  prev_diff  prev_diff >= thres  diff < thres  prev_diff >= thres && diff < thres
        // 0
        // 0  0         0     0          0                   1             0
        // 1
        // 1  1         1     0          0                   0             0
        // 1
        // 1  1         0     1          1                   1             1 -> end segment at position 3
        // 2
        // 2  2         1     0          0                   0             0
        // 2
        // 2  2         0     1          1                   1             1 -> end segment at position 7
        // 3
        // 3
        // -> end segment at position 12

        let scan_line = new_vertical_scan_line(
            &image,
            &field_color,
            1,
            2,
            EdgeDetectionSourceParameters::Luminance,
            1,
            MedianModeParameters::ThreePixels,
            0.0,
            &[],
        );
        assert_eq!(scan_line.position, 1);
        assert_eq!(scan_line.segments.len(), 3);
        assert_eq!(scan_line.segments[0].start, 0);
        assert_eq!(scan_line.segments[0].end, 3);
        assert_eq!(scan_line.segments[0].start_edge_type, EdgeType::ImageBorder);
        assert_eq!(scan_line.segments[0].end_edge_type, EdgeType::Rising);
        assert_eq!(scan_line.segments[1].start, 3);
        assert_eq!(scan_line.segments[1].end, 7);
        assert_eq!(scan_line.segments[1].start_edge_type, EdgeType::Rising);
        assert_eq!(scan_line.segments[1].end_edge_type, EdgeType::Rising);
        assert_eq!(scan_line.segments[2].start, 7);
        assert_eq!(scan_line.segments[2].end, 12);
        assert_eq!(scan_line.segments[2].start_edge_type, EdgeType::Rising);
        assert_eq!(scan_line.segments[2].end_edge_type, EdgeType::ImageBorder);
    }

    #[test]
    fn image_with_three_vertical_decreasing_segments_without_median() {
        let image = YCbCr422Image::from_ycbcr_buffer(
            1,
            12,
            vec![
                // only evaluating every secondth 422 pixel
                YCbCr422::new(3, 0, 0, 0),
                YCbCr422::new(3, 0, 0, 0), // skipped
                // segment boundary will be here
                YCbCr422::new(2, 0, 0, 0),
                YCbCr422::new(2, 0, 0, 0), // skipped
                YCbCr422::new(2, 0, 0, 0),
                YCbCr422::new(2, 0, 0, 0), // skipped
                // segment boundary will be here
                YCbCr422::new(1, 0, 0, 0),
                YCbCr422::new(1, 0, 0, 0), // skipped
                YCbCr422::new(1, 0, 0, 0),
                YCbCr422::new(1, 0, 0, 0), // skipped
                YCbCr422::new(0, 0, 0, 0),
                YCbCr422::new(0, 0, 0, 0), // skipped

                                           // segment boundary will be here
            ],
        );
        let field_color = FieldColor {
            red_chromaticity_threshold: 0.37,
            blue_chromaticity_threshold: 0.38,
            green_chromaticity_threshold: 0.43,
            green_luminance_threshold: 255.0,
            luminance_threshold: 25.0,
            function: FieldColorFunction::GreenChromaticity,
            hue_low_threshold: 0.0,
            hue_high_threshold: 0.0,
            saturation_low_threshold: 0.0,
            saturation_high_threshold: 0.0,
        };

        // y  diff  prev_diff  prev_diff <= -thres  diff > -thres  prev_diff <= -thres && diff > -thres
        // 3   0     0         0                    1              0
        // 2  -1     0         0                    0              0
        // 2   0    -1         1                    1              1 -> end segment at position 2
        // 1  -1     0         0                    0              0
        // 1   0    -1         1                    1              1 -> end segment at position 6
        // 0  -1     0         0                    0              0
        // -> end segment at position 12

        let scan_line = new_vertical_scan_line(
            &image,
            &field_color,
            0,
            2,
            EdgeDetectionSourceParameters::Luminance,
            1,
            MedianModeParameters::Disabled,
            0.0,
            &[],
        );
        assert_eq!(scan_line.position, 0);
        assert_eq!(scan_line.segments.len(), 3);
        assert_eq!(scan_line.segments[0].start, 0);
        assert_eq!(scan_line.segments[0].end, 2);
        assert_eq!(scan_line.segments[0].start_edge_type, EdgeType::ImageBorder);
        assert_eq!(scan_line.segments[0].end_edge_type, EdgeType::Falling);
        assert_eq!(scan_line.segments[1].start, 2);
        assert_eq!(scan_line.segments[1].end, 6);
        assert_eq!(scan_line.segments[1].start_edge_type, EdgeType::Falling);
        assert_eq!(scan_line.segments[1].end_edge_type, EdgeType::Falling);
        assert_eq!(scan_line.segments[2].start, 6);
        assert_eq!(scan_line.segments[2].end, 12);
        assert_eq!(scan_line.segments[2].start_edge_type, EdgeType::Falling);
        assert_eq!(scan_line.segments[2].end_edge_type, EdgeType::ImageBorder);
    }

    #[test]
    fn image_with_three_vertical_decreasing_segments_with_median() {
        let row = |y| {
            [
                YCbCr422::new(y, 0, 0, 0),
                YCbCr422::new(y, 0, 0, 0),
                YCbCr422::new(y, 0, 0, 0),
            ]
        };
        let image = YCbCr422Image::from_ycbcr_buffer(
            3,
            12,
            [
                // only evaluating every secondth 422 pixel
                row(3),
                row(3), // skipped
                row(2),
                // segment boundary will be here
                row(2), // skipped
                row(2),
                row(2), // skipped
                row(1),
                // segment boundary will be here
                row(1), // skipped
                row(1),
                row(1), // skipped
                row(0),
                row(0), // skipped

                        // segment boundary will be here
            ]
            .into_iter()
            .flatten()
            .collect(),
        );
        let field_color = FieldColor {
            red_chromaticity_threshold: 0.37,
            blue_chromaticity_threshold: 0.38,
            green_chromaticity_threshold: 0.43,
            green_luminance_threshold: 255.0,
            luminance_threshold: 25.0,
            function: FieldColorFunction::GreenChromaticity,
            hue_low_threshold: 0.0,
            hue_high_threshold: 0.0,
            saturation_low_threshold: 0.0,
            saturation_high_threshold: 0.0,
        };

        // y  y_median  diff  prev_diff  prev_diff <= -thres  diff > -thres  prev_diff <= -thres && diff > -thres
        // 3
        // 3  3   0     0         0                    1              0
        // 2
        // 2  2  -1     0         0                    0              0
        // 2
        // 2  2   0    -1         1                    1              1 -> end segment at position 3
        // 1
        // 1  1  -1     0         0                    0              0
        // 1
        // 1  1   0    -1         1                    1              1 -> end segment at position 7
        // 0
        // 0
        // -> end segment at position 12

        let scan_line = new_vertical_scan_line(
            &image,
            &field_color,
            1,
            2,
            EdgeDetectionSourceParameters::Luminance,
            1,
            MedianModeParameters::ThreePixels,
            0.0,
            &[],
        );
        assert_eq!(scan_line.position, 1);
        assert_eq!(scan_line.segments.len(), 3);
        assert_eq!(scan_line.segments[0].start, 0);
        assert_eq!(scan_line.segments[0].end, 3);
        assert_eq!(scan_line.segments[0].start_edge_type, EdgeType::ImageBorder);
        assert_eq!(scan_line.segments[0].end_edge_type, EdgeType::Falling);
        assert_eq!(scan_line.segments[1].start, 3);
        assert_eq!(scan_line.segments[1].end, 7);
        assert_eq!(scan_line.segments[1].start_edge_type, EdgeType::Falling);
        assert_eq!(scan_line.segments[1].end_edge_type, EdgeType::Falling);
        assert_eq!(scan_line.segments[2].start, 7);
        assert_eq!(scan_line.segments[2].end, 12);
        assert_eq!(scan_line.segments[2].start_edge_type, EdgeType::Falling);
        assert_eq!(scan_line.segments[2].end_edge_type, EdgeType::ImageBorder);
    }

    #[test]
    fn image_with_three_vertical_segments_with_higher_differences_without_median() {
        let image = YCbCr422Image::from_ycbcr_buffer(
            1,
            44,
            vec![
                // only evaluating every secondth 422 pixel
                YCbCr422::new(0, 0, 0, 0),
                YCbCr422::new(0, 0, 0, 0), // skipped
                YCbCr422::new(0, 0, 0, 0),
                YCbCr422::new(0, 0, 0, 0), // skipped
                YCbCr422::new(0, 0, 0, 0),
                YCbCr422::new(0, 0, 0, 0), // skipped
                YCbCr422::new(0, 0, 0, 0),
                YCbCr422::new(0, 0, 0, 0), // skipped
                YCbCr422::new(1, 0, 0, 0),
                YCbCr422::new(1, 0, 0, 0), // skipped
                YCbCr422::new(2, 0, 0, 0),
                YCbCr422::new(2, 0, 0, 0), // skipped
                YCbCr422::new(3, 0, 0, 0),
                YCbCr422::new(3, 0, 0, 0), // skipped
                YCbCr422::new(4, 0, 0, 0),
                YCbCr422::new(4, 0, 0, 0), // skipped
                // segment boundary will be here
                YCbCr422::new(5, 0, 0, 0),
                YCbCr422::new(5, 0, 0, 0), // skipped
                YCbCr422::new(4, 0, 0, 0),
                YCbCr422::new(4, 0, 0, 0), // skipped
                YCbCr422::new(3, 0, 0, 0),
                YCbCr422::new(3, 0, 0, 0), // skipped
                YCbCr422::new(2, 0, 0, 0),
                YCbCr422::new(2, 0, 0, 0), // skipped
                YCbCr422::new(1, 0, 0, 0),
                YCbCr422::new(1, 0, 0, 0), // skipped
                // segment boundary will be here
                YCbCr422::new(0, 0, 0, 0),
                YCbCr422::new(0, 0, 0, 0), // skipped
                YCbCr422::new(0, 0, 0, 0),
                YCbCr422::new(0, 0, 0, 0), // skipped
                YCbCr422::new(0, 0, 0, 0),
                YCbCr422::new(0, 0, 0, 0), // skipped
                YCbCr422::new(0, 0, 0, 0),
                YCbCr422::new(0, 0, 0, 0), // skipped
                YCbCr422::new(0, 0, 0, 0),
                YCbCr422::new(0, 0, 0, 0), // skipped
                YCbCr422::new(0, 0, 0, 0),
                YCbCr422::new(0, 0, 0, 0), // skipped
                YCbCr422::new(0, 0, 0, 0),
                YCbCr422::new(0, 0, 0, 0), // skipped
                YCbCr422::new(0, 0, 0, 0),
                YCbCr422::new(0, 0, 0, 0), // skipped
                YCbCr422::new(0, 0, 0, 0),
                YCbCr422::new(0, 0, 0, 0), // skipped

                                           // segment boundary will be here
            ],
        );
        let field_color = FieldColor {
            red_chromaticity_threshold: 0.37,
            blue_chromaticity_threshold: 0.38,
            green_chromaticity_threshold: 0.43,
            green_luminance_threshold: 255.0,
            luminance_threshold: 25.0,
            function: FieldColorFunction::GreenChromaticity,
            hue_low_threshold: 0.0,
            hue_high_threshold: 0.0,
            saturation_low_threshold: 0.0,
            saturation_high_threshold: 0.0,
        };

        // y  diff  prev_diff  prev_diff >= thres  diff < thres  prev_diff >= thres && diff < thres
        // 0   0     0         0                   0             0
        // 0   0     0         0                   0             0
        // 0   0     0         0                   0             0
        // 0   0     0         0                   0             0
        // 1   1     0         0                   0             0
        // 2   1     1         1                   0             0
        // 3   1     1         1                   0             0
        // 4   1     1         1                   0             0
        // 5   1     1         1                   0             0
        // 4  -1     1         1                   1             1 -> end segment at position 16
        // 3  -1    -1         0                   1             0
        // 2  -1    -1         0                   1             0
        // 1  -1    -1         0                   1             0
        // 0  -1    -1         0                   1             0
        // 0   0    -1         0                   0             0
        // 0   0     0         0                   0             0
        // 0   0     0         0                   0             0
        // 0   0     0         0                   0             0
        // 0   0     0         0                   0             0
        // 0   0     0         0                   0             0
        // 0   0     0         0                   0             0
        // 0   0     0         0                   0             0
        // -> end segment at position 44

        // y  diff  prev_diff  prev_diff <= -thres  diff > -thres  prev_diff <= -thres && diff > -thres
        // 0   0     0         0                    1              0
        // 0   0     0         0                    1              0
        // 0   0     0         0                    1              0
        // 0   0     0         0                    1              0
        // 1   1     0         0                    1              0
        // 2   1     1         0                    1              0
        // 3   1     1         0                    1              0
        // 4   1     1         0                    1              0
        // 5   1     1         0                    1              0
        // 4  -1     1         0                    0              0
        // 3  -1    -1         1                    0              0
        // 2  -1    -1         1                    0              0
        // 1  -1    -1         1                    0              0
        // 0  -1    -1         1                    0              0
        // 0   0    -1         1                    1              1 -> end segment at position 26
        // 0   0     0         0                    1              0
        // 0   0     0         0                    1              0
        // 0   0     0         0                    1              0
        // 0   0     0         0                    1              0
        // 0   0     0         0                    1              0
        // 0   0     0         0                    1              0
        // 0   0     0         0                    1              0
        // -> end segment at position 44

        let scan_line = new_vertical_scan_line(
            &image,
            &field_color,
            0,
            2,
            EdgeDetectionSourceParameters::Luminance,
            1,
            MedianModeParameters::Disabled,
            0.0,
            &[],
        );
        assert_eq!(scan_line.position, 0);
        assert_eq!(scan_line.segments.len(), 3);
        assert_eq!(scan_line.segments[0].start, 0);
        assert_eq!(scan_line.segments[0].end, 16);
        assert_eq!(scan_line.segments[0].start_edge_type, EdgeType::ImageBorder);
        assert_eq!(scan_line.segments[0].end_edge_type, EdgeType::Rising);
        assert_eq!(scan_line.segments[1].start, 16);
        assert_eq!(scan_line.segments[1].end, 26);
        assert_eq!(scan_line.segments[1].start_edge_type, EdgeType::Rising);
        assert_eq!(scan_line.segments[1].end_edge_type, EdgeType::Falling);
        assert_eq!(scan_line.segments[2].start, 26);
        assert_eq!(scan_line.segments[2].end, 44);
        assert_eq!(scan_line.segments[2].start_edge_type, EdgeType::Falling);
        assert_eq!(scan_line.segments[2].end_edge_type, EdgeType::ImageBorder);
    }

    #[test]
    fn image_with_three_vertical_segments_with_higher_differences_with_median() {
        let row = |y| {
            [
                YCbCr422::new(y, 0, 0, 0),
                YCbCr422::new(y, 0, 0, 0),
                YCbCr422::new(y, 0, 0, 0),
            ]
        };
        let image = YCbCr422Image::from_ycbcr_buffer(
            3,
            44,
            [
                // only evaluating every secondth 422 pixel
                row(0),
                row(0), // skipped
                row(0),
                row(0), // skipped
                row(0),
                row(0), // skipped
                row(0),
                row(0), // skipped
                row(1),
                row(1), // skipped
                row(2),
                row(2), // skipped
                row(3),
                row(3), // skipped
                row(4),
                row(4), // skipped
                row(5),
                // segment boundary will be here
                row(5), // skipped
                row(4),
                row(4), // skipped
                row(3),
                row(3), // skipped
                row(2),
                row(2), // skipped
                row(1),
                row(1), // skipped
                row(0),
                // segment boundary will be here
                row(0), // skipped
                row(0),
                row(0), // skipped
                row(0),
                row(0), // skipped
                row(0),
                row(0), // skipped
                row(0),
                row(0), // skipped
                row(0),
                row(0), // skipped
                row(0),
                row(0), // skipped
                row(0),
                row(0), // skipped
                row(0),
                row(0), // skipped

                        // segment boundary will be here
            ]
            .into_iter()
            .flatten()
            .collect(),
        );
        let field_color = FieldColor {
            red_chromaticity_threshold: 0.37,
            blue_chromaticity_threshold: 0.38,
            green_chromaticity_threshold: 0.43,
            green_luminance_threshold: 255.0,
            luminance_threshold: 25.0,
            function: FieldColorFunction::GreenChromaticity,
            hue_low_threshold: 0.0,
            hue_high_threshold: 0.0,
            saturation_low_threshold: 0.0,
            saturation_high_threshold: 0.0,
        };

        // y  y_median  diff  prev_diff  prev_diff >= thres  diff < thres  prev_diff >= thres && diff < thres
        // 0
        // 0  0          0     0         0                   0             0
        // 0
        // 0  0          0     0         0                   0             0
        // 0
        // 0  0          0     0         0                   0             0
        // 0
        // 0  0          0     0         0                   0             0
        // 1
        // 1  1          1     0         0                   0             0
        // 2
        // 2  2          1     1         1                   0             0
        // 3
        // 3  3          1     1         1                   0             0
        // 4
        // 4  4          1     1         1                   0             0
        // 5
        // 5  5          1     1         1                   0             0
        // 4
        // 4  4         -1     1         1                   1             1 -> end segment at position 17
        // 3
        // 3  3         -1    -1         0                   1             0
        // 2
        // 2  2         -1    -1         0                   1             0
        // 1
        // 1  1         -1    -1         0                   1             0
        // 0
        // 0  0         -1    -1         0                   1             0
        // 0
        // 0  0          0    -1         0                   0             0
        // 0
        // 0  0          0     0         0                   0             0
        // 0
        // 0  0          0     0         0                   0             0
        // 0
        // 0  0          0     0         0                   0             0
        // 0
        // 0  0          0     0         0                   0             0
        // 0
        // 0  0          0     0         0                   0             0
        // 0
        // 0  0          0     0         0                   0             0
        // 0
        // 0
        // -> end segment at position 44

        // y  y_median  diff  prev_diff  prev_diff <= -thres  diff > -thres  prev_diff <= -thres && diff > -thres
        // 0
        // 0  0          0     0         0                    1              0
        // 0
        // 0  0          0     0         0                    1              0
        // 0
        // 0  0          0     0         0                    1              0
        // 0
        // 0  0          0     0         0                    1              0
        // 1
        // 1  1          1     0         0                    1              0
        // 2
        // 2  2          1     1         0                    1              0
        // 3
        // 3  3          1     1         0                    1              0
        // 4
        // 4  4          1     1         0                    1              0
        // 5
        // 5  5          1     1         0                    1              0
        // 4
        // 4  4         -1     1         0                    0              0
        // 3
        // 3  3         -1    -1         1                    0              0
        // 2
        // 2  2         -1    -1         1                    0              0
        // 1
        // 1  1         -1    -1         1                    0              0
        // 0
        // 0  0         -1    -1         1                    0              0
        // 0
        // 0  0          0    -1         1                    1              1 -> end segment at position 27
        // 0
        // 0  0          0     0         0                    1              0
        // 0
        // 0  0          0     0         0                    1              0
        // 0
        // 0  0          0     0         0                    1              0
        // 0
        // 0  0          0     0         0                    1              0
        // 0
        // 0  0          0     0         0                    1              0
        // 0
        // 0  0          0     0         0                    1              0
        // 0
        // 0
        // -> end segment at position 44

        let scan_line = new_vertical_scan_line(
            &image,
            &field_color,
            1,
            2,
            EdgeDetectionSourceParameters::Luminance,
            1,
            MedianModeParameters::ThreePixels,
            0.0,
            &[],
        );
        assert_eq!(scan_line.position, 1);
        assert_eq!(scan_line.segments.len(), 3);
        assert_eq!(scan_line.segments[0].start, 0);
        assert_eq!(scan_line.segments[0].end, 17);
        assert_eq!(scan_line.segments[0].start_edge_type, EdgeType::ImageBorder);
        assert_eq!(scan_line.segments[0].end_edge_type, EdgeType::Rising);
        assert_eq!(scan_line.segments[1].start, 17);
        assert_eq!(scan_line.segments[1].end, 27);
        assert_eq!(scan_line.segments[1].start_edge_type, EdgeType::Rising);
        assert_eq!(scan_line.segments[1].end_edge_type, EdgeType::Falling);
        assert_eq!(scan_line.segments[2].start, 27);
        assert_eq!(scan_line.segments[2].end, 44);
        assert_eq!(scan_line.segments[2].start_edge_type, EdgeType::Falling);
        assert_eq!(scan_line.segments[2].end_edge_type, EdgeType::ImageBorder);
    }

    #[test]
    fn image_with_one_vertical_segment_with_increasing_differences_without_median() {
        let image = YCbCr422Image::from_ycbcr_buffer(
            1,
            16,
            vec![
                // only evaluating every secondth 422 pixel
                YCbCr422::new(0, 0, 0, 0),
                YCbCr422::new(0, 0, 0, 0), // skipped
                YCbCr422::new(0, 0, 0, 0),
                YCbCr422::new(0, 0, 0, 0), // skipped
                YCbCr422::new(1, 0, 0, 0),
                YCbCr422::new(1, 0, 0, 0), // skipped
                YCbCr422::new(3, 0, 0, 0),
                YCbCr422::new(3, 0, 0, 0), // skipped
                YCbCr422::new(6, 0, 0, 0),
                YCbCr422::new(6, 0, 0, 0), // skipped
                YCbCr422::new(10, 0, 0, 0),
                YCbCr422::new(10, 0, 0, 0), // skipped
                YCbCr422::new(15, 0, 0, 0),
                YCbCr422::new(15, 0, 0, 0), // skipped
                YCbCr422::new(21, 0, 0, 0),
                YCbCr422::new(21, 0, 0, 0), // skipped

                                            // segment boundary will be here
            ],
        );
        let field_color = FieldColor {
            red_chromaticity_threshold: 0.37,
            blue_chromaticity_threshold: 0.38,
            green_chromaticity_threshold: 0.43,
            green_luminance_threshold: 255.0,
            luminance_threshold: 25.0,
            function: FieldColorFunction::GreenChromaticity,
            hue_low_threshold: 0.0,
            hue_high_threshold: 0.0,
            saturation_low_threshold: 0.0,
            saturation_high_threshold: 0.0,
        };

        //  y  diff  prev_diff  prev_diff >= thres  diff < thres  prev_diff >= thres && diff < thres
        //  0  0     0          0                   1             0
        //  0  0     0          0                   1             0
        //  1  1     0          0                   0             0
        //  3  2     1          1                   0             0
        //  6  3     2          1                   0             0
        // 10  4     3          1                   0             0
        // 15  5     4          1                   0             0
        // 21  6     5          1                   0             0
        // -> end segment at position 16

        let scan_line = new_vertical_scan_line(
            &image,
            &field_color,
            0,
            2,
            EdgeDetectionSourceParameters::Luminance,
            1,
            MedianModeParameters::Disabled,
            0.0,
            &[],
        );
        assert_eq!(scan_line.position, 0);
        assert_eq!(scan_line.segments.len(), 1);
        assert_eq!(scan_line.segments[0].start, 0);
        assert_eq!(scan_line.segments[0].end, 16);
        assert_eq!(scan_line.segments[0].start_edge_type, EdgeType::ImageBorder);
        assert_eq!(scan_line.segments[0].end_edge_type, EdgeType::ImageBorder);
    }

    #[test]
    fn image_with_one_vertical_segment_with_increasing_differences_with_median() {
        let row = |y| {
            [
                YCbCr422::new(y, 0, 0, 0),
                YCbCr422::new(y, 0, 0, 0),
                YCbCr422::new(y, 0, 0, 0),
            ]
        };
        let image = YCbCr422Image::from_ycbcr_buffer(
            3,
            16,
            [
                // only evaluating every secondth 422 pixel
                row(0),
                row(0), // skipped
                row(0),
                row(0), // skipped
                row(1),
                row(1), // skipped
                row(3),
                row(3), // skipped
                row(6),
                row(6), // skipped
                row(10),
                row(10), // skipped
                row(15),
                row(15), // skipped
                row(21),
                row(21), // skipped
                         // segment boundary will be here
            ]
            .into_iter()
            .flatten()
            .collect(),
        );
        let field_color = FieldColor {
            red_chromaticity_threshold: 0.37,
            blue_chromaticity_threshold: 0.38,
            green_chromaticity_threshold: 0.43,
            green_luminance_threshold: 255.0,
            luminance_threshold: 25.0,
            function: FieldColorFunction::GreenChromaticity,
            hue_low_threshold: 0.0,
            hue_high_threshold: 0.0,
            saturation_low_threshold: 0.0,
            saturation_high_threshold: 0.0,
        };

        //  y  y_median  diff  prev_diff  prev_diff >= thres  diff < thres  prev_diff >= thres && diff < thres
        //  0
        //  0   0        0     0          0                   1             0
        //  0
        //  0   0        0     0          0                   1             0
        //  1
        //  1   1        1     0          0                   0             0
        //  3
        //  3   3        2     1          1                   0             0
        //  6
        //  6   6        3     2          1                   0             0
        // 10
        // 10  10        4     3          1                   0             0
        // 15
        // 15  15        5     4          1                   0             0
        // 21
        // 21
        // -> end segment at position 16

        let scan_line = new_vertical_scan_line(
            &image,
            &field_color,
            1,
            2,
            EdgeDetectionSourceParameters::Luminance,
            1,
            MedianModeParameters::ThreePixels,
            0.0,
            &[],
        );
        assert_eq!(scan_line.position, 1);
        assert_eq!(scan_line.segments.len(), 1);
        assert_eq!(scan_line.segments[0].start, 0);
        assert_eq!(scan_line.segments[0].end, 16);
        assert_eq!(scan_line.segments[0].start_edge_type, EdgeType::ImageBorder);
        assert_eq!(scan_line.segments[0].end_edge_type, EdgeType::ImageBorder);
    }

    #[test]
    fn median_of_three_with_same_values() {
        // first == second == third
        assert_eq!(median_of_three([0, 0, 0]), 0);
        // first < second == third
        assert_eq!(median_of_three([0, 1, 1]), 1);
        // first == second < third
        assert_eq!(median_of_three([0, 0, 1]), 0);
        // first == third < second
        assert_eq!(median_of_three([0, 1, 0]), 0);
    }

    #[test]
    fn median_of_three_with_different_values() {
        // first <= second <= third
        assert_eq!(median_of_three([0, 1, 2]), 1);
        // first <= third < second
        assert_eq!(median_of_three([0, 2, 1]), 1);
        // third < first <= second
        assert_eq!(median_of_three([1, 2, 0]), 1);
        // second < first <= third
        assert_eq!(median_of_three([1, 0, 2]), 1);
        // second <= third < first
        assert_eq!(median_of_three([2, 0, 1]), 1);
        // third < second <= first
        assert_eq!(median_of_three([2, 1, 0]), 1);
    }

    #[test]
    fn median_of_five_calculates_median() {
        for (first, second, third, fourth, fifth) in iproduct!(0..5, 0..5, 0..5, 0..5, 0..5) {
            let calculated_median = median_of_five([first, second, third, fourth, fifth]);
            let mut numbers = [first, second, third, fourth, fifth];
            numbers.sort();
            let real_median = numbers[2];
            assert_eq!(calculated_median,real_median, "test_case: {first} {second} {third} {fourth} {fifth}, median_of_five: {calculated_median}");
        }
    }
}
