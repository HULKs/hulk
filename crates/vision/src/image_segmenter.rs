use std::ops::Add;

use color_eyre::Result;
use projection::{camera_matrix::CameraMatrix, horizon::Horizon, Projection};
use serde::{Deserialize, Serialize};

use context_attribute::context;
use coordinate_systems::{Ground, Pixel};
use framework::MainOutput;
use linear_algebra::{point, vector, Framed, Point2, Vector2};
use types::{
    color::{Intensity, Rgb, YCbCr444},
    image_segments::{Direction, EdgeType, ImageSegments, ScanGrid, ScanLine, Segment},
    limb::project_onto_limbs,
    limb::{Limb, ProjectedLimbs},
    parameters::MedianModeParameters,
    ycbcr422_image::YCbCr422Image,
};

use crate::field_color_tree::{self, Features};

#[derive(Deserialize, Serialize)]
pub struct ImageSegmenter {}

#[context]
pub struct CreationContext {}

#[context]
pub struct CycleContext {
    image: Input<YCbCr422Image, "image">,

    camera_matrix: RequiredInput<Option<CameraMatrix>, "camera_matrix?">,
    projected_limbs: Input<Option<ProjectedLimbs>, "projected_limbs?">,

    horizontal_stride: Parameter<usize, "image_segmenter.$cycler_instance.horizontal_stride">,
    vertical_stride_in_ground: Parameter<
        Framed<Ground, f32>,
        "image_segmenter.$cycler_instance.vertical_stride_in_ground",
    >,
    horizontal_edge_threshold:
        Parameter<u8, "image_segmenter.$cycler_instance.horizontal_edge_threshold">,
    horizontal_median_mode:
        Parameter<MedianModeParameters, "image_segmenter.$cycler_instance.horizontal_median_mode">,

    vertical_stride: Parameter<usize, "image_segmenter.$cycler_instance.vertical_stride">,
    vertical_edge_threshold:
        Parameter<u8, "image_segmenter.$cycler_instance.vertical_edge_threshold">,
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
        Ok(Self {})
    }

    pub fn cycle(&mut self, context: CycleContext) -> Result<MainOutputs> {
        let projected_limbs = context
            .projected_limbs
            .map_or(Default::default(), |projected_limbs| {
                projected_limbs.limbs.as_slice()
            });

        let horizon = context
            .camera_matrix
            .horizon
            .unwrap_or(Horizon::ABOVE_IMAGE);

        let scan_grid = new_grid(
            context.image,
            context.camera_matrix,
            &horizon,
            *context.horizontal_stride,
            *context.horizontal_edge_threshold as i16,
            *context.horizontal_median_mode,
            *context.vertical_stride,
            *context.vertical_stride_in_ground,
            *context.vertical_edge_threshold as i16,
            *context.vertical_median_mode,
            projected_limbs,
        );
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

fn find_minimum_y_on_limbs(image: &YCbCr422Image, projected_limbs: &[Limb]) -> u32 {
    let limb_pixels = projected_limbs
        .iter()
        .flat_map(|limb| &limb.pixel_polyline)
        .map(|point| point.map(|x| x as u32));
    let inside_image = limb_pixels.filter(|point| (0..image.width()).contains(&point.x()));
    // minimum means higher in the image, the minimum in y is the top-most pixel
    let minimum_y = inside_image
        .map(|point| point.y())
        .min()
        .unwrap_or(image.height());
    minimum_y.clamp(0, image.height())
}

#[allow(clippy::too_many_arguments)]
fn new_grid(
    image: &YCbCr422Image,
    camera_matrix: &CameraMatrix,
    horizon: &Horizon,
    horizontal_stride: usize,
    horizontal_edge_threshold: i16,
    horizontal_median_mode: MedianModeParameters,
    vertical_stride: usize,
    vertical_stride_in_ground: Framed<Ground, f32>,
    vertical_edge_threshold: i16,
    vertical_median_mode: MedianModeParameters,
    projected_limbs: &[Limb],
) -> ScanGrid {
    let horizontal_padding_size = padding_size(horizontal_median_mode);
    let vertical_padding_size = padding_size(vertical_median_mode);

    let horizon_y_maximum = (horizon.horizon_y_maximum() as u32).clamp(0, image.height());
    let limbs_y_minimum = find_minimum_y_on_limbs(image, projected_limbs);

    let horizontal_scan_lines = match horizontal_median_mode {
        MedianModeParameters::Disabled => collect_horizontal_scan_lines::<MedianMode<0>>(
            image,
            camera_matrix,
            horizontal_stride,
            horizontal_edge_threshold,
            vertical_stride_in_ground,
            horizontal_padding_size,
            horizon_y_maximum,
            limbs_y_minimum,
        ),
        MedianModeParameters::ThreePixels => collect_horizontal_scan_lines::<MedianMode<3>>(
            image,
            camera_matrix,
            horizontal_stride,
            horizontal_edge_threshold,
            vertical_stride_in_ground,
            horizontal_padding_size,
            horizon_y_maximum,
            limbs_y_minimum,
        ),
        MedianModeParameters::FivePixels => collect_horizontal_scan_lines::<MedianMode<5>>(
            image,
            camera_matrix,
            horizontal_stride,
            horizontal_edge_threshold,
            vertical_stride_in_ground,
            horizontal_padding_size,
            horizon_y_maximum,
            limbs_y_minimum,
        ),
    };
    let vertical_scan_lines = match vertical_median_mode {
        MedianModeParameters::Disabled => collect_vertical_scan_lines::<MedianMode<0>>(
            image,
            horizon,
            horizontal_stride,
            vertical_stride,
            vertical_edge_threshold,
            projected_limbs,
            vertical_padding_size,
        ),
        MedianModeParameters::ThreePixels => collect_vertical_scan_lines::<MedianMode<3>>(
            image,
            horizon,
            horizontal_stride,
            vertical_stride,
            vertical_edge_threshold,
            projected_limbs,
            vertical_padding_size,
        ),
        MedianModeParameters::FivePixels => collect_vertical_scan_lines::<MedianMode<5>>(
            image,
            horizon,
            horizontal_stride,
            vertical_stride,
            vertical_edge_threshold,
            projected_limbs,
            vertical_padding_size,
        ),
    };
    ScanGrid {
        horizontal_scan_lines,
        vertical_scan_lines,
    }
}

fn collect_vertical_scan_lines<MedianMode: MedianSampling>(
    image: &YCbCr422Image,
    horizon: &Horizon,
    horizontal_stride: usize,
    vertical_stride: usize,
    vertical_edge_threshold: i16,
    projected_limbs: &[Limb],
    vertical_padding_size: u32,
) -> Vec<ScanLine> {
    (vertical_padding_size..image.width() - vertical_padding_size)
        .step_by(horizontal_stride)
        .map(|x| {
            let horizon_y = horizon.y_at_x(x as f32).clamp(0.0, image.height() as f32);
            new_vertical_scan_line::<MedianMode>(
                image,
                x,
                vertical_stride,
                vertical_edge_threshold,
                horizon_y,
                projected_limbs,
            )
        })
        .collect()
}

#[allow(clippy::too_many_arguments)]
fn collect_horizontal_scan_lines<MedianMode: MedianSampling>(
    image: &YCbCr422Image,
    camera_matrix: &CameraMatrix,
    horizontal_stride: usize,
    horizontal_edge_threshold: i16,
    vertical_stride: Framed<Ground, f32>,
    horizontal_padding_size: u32,
    horizon_y_maximum: u32,
    limbs_y_minimum: u32,
) -> Vec<ScanLine> {
    let mut horizontal_scan_lines = vec![];
    // do not start at horizon because of numerically unstable math
    let mut y = horizon_y_maximum + 1 + horizontal_padding_size;

    while y + horizontal_padding_size < limbs_y_minimum {
        horizontal_scan_lines.push(new_horizontal_scan_line::<MedianMode>(
            image,
            y,
            horizontal_stride,
            horizontal_edge_threshold,
        ));

        y = next_horizontal_segment_y(image, camera_matrix, vertical_stride, y)
            .unwrap_or(0)
            .max(y + 4);
    }
    horizontal_scan_lines
}

fn next_horizontal_segment_y(
    image: &YCbCr422Image,
    camera_matrix: &CameraMatrix,
    vertical_stride: Framed<Ground, f32>,
    y: u32,
) -> Option<u32> {
    let center_at_y = point![image.width() / 2, y].map(|x| x as f32);
    let center_in_ground = camera_matrix.pixel_to_ground(center_at_y).ok()?;

    let vertical_stride = vector![-vertical_stride.inner, 0.0];
    let next_in_pixel = camera_matrix
        .ground_to_pixel(center_in_ground + vertical_stride)
        .ok()?;

    Some(next_in_pixel.y() as u32)
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

trait Median<T> {
    fn median(self) -> T;
}

impl<T> Median<T> for [T; 3]
where
    T: Ord + Copy,
{
    fn median(mut self) -> T {
        let (_, median, _) = self.select_nth_unstable(1);
        *median
    }
}

impl<T> Median<T> for [T; 5]
where
    T: Ord + Copy,
{
    fn median(mut self) -> T {
        let (_, median, _) = self.select_nth_unstable(2);
        *median
    }
}

fn new_horizontal_scan_line<MedianMode: MedianSampling>(
    image: &YCbCr422Image,
    position: u32,
    stride: usize,
    edge_threshold: i16,
) -> ScanLine {
    let start_x = 0;
    let end_x = image.width();

    let edge_detection_value =
        MedianMode::sample(point![start_x, position], Direction::Horizontal, image);
    let mut state = ScanLineState::new(
        edge_detection_value as i16,
        start_x as u16,
        EdgeType::ImageBorder,
    );

    let mut segments = Vec::with_capacity((end_x - start_x) as usize / stride);

    for x in (start_x..end_x).step_by(stride) {
        let edge_detection_value =
            MedianMode::sample(point![x, position], Direction::Horizontal, image);

        if let Some(mut segment) = detect_edge(
            &mut state,
            x as u16,
            edge_detection_value as i16,
            edge_threshold,
        ) {
            segment.color =
                average_color_in_segment(&segment, position, Direction::Horizontal, image);
            segment.field_color =
                detect_field_color_in_segment(&segment, position, Direction::Horizontal, image);
            segments.push(segment);
        }
    }

    let mut last_segment = Segment {
        start: state.start_position,
        end: image.width() as u16,
        start_edge_type: state.start_edge_type,
        end_edge_type: EdgeType::ImageBorder,
        color: Default::default(),
        field_color: Intensity::Low,
    };
    last_segment.color =
        average_color_in_segment(&last_segment, position, Direction::Horizontal, image);
    last_segment.field_color =
        detect_field_color_in_segment(&last_segment, position, Direction::Horizontal, image);
    segments.push(last_segment);

    ScanLine {
        position: position as u16,
        segments,
    }
}

#[allow(clippy::too_many_arguments)]
fn new_vertical_scan_line<MedianMode: MedianSampling>(
    image: &YCbCr422Image,
    position: u32,
    stride: usize,
    edge_threshold: i16,
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

    let edge_detection_value =
        MedianMode::sample(point![position, start_y], Direction::Vertical, image);
    let mut state = ScanLineState::new(
        edge_detection_value as i16,
        start_y as u16,
        EdgeType::ImageBorder,
    );

    let mut segments = Vec::with_capacity((end_y - start_y) as usize / stride);
    for y in (start_y..end_y).step_by(stride) {
        let edge_detection_value =
            MedianMode::sample(point![position, y], Direction::Vertical, image);

        if let Some(mut segment) = detect_edge(
            &mut state,
            y as u16,
            edge_detection_value as i16,
            edge_threshold,
        ) {
            if segment_is_below_limbs(position as u16, &segment, projected_limbs) {
                fix_previous_edge_type(&mut segments);
                break;
            }
            segment.color =
                average_color_in_segment(&segment, position, Direction::Vertical, image);
            segment.field_color =
                detect_field_color_in_segment(&segment, position, Direction::Vertical, image);
            segments.push(segment);
        }
    }

    let mut last_segment = Segment {
        start: state.start_position,
        end: image.height() as u16,
        start_edge_type: state.start_edge_type,
        end_edge_type: EdgeType::ImageBorder,
        color: Default::default(),
        field_color: Intensity::Low,
    };
    if !segment_is_below_limbs(position as u16, &last_segment, projected_limbs) {
        last_segment.color =
            average_color_in_segment(&last_segment, position, Direction::Vertical, image);
        last_segment.field_color =
            detect_field_color_in_segment(&last_segment, position, Direction::Vertical, image);
        segments.push(last_segment);
    }

    ScanLine {
        position: position as u16,
        segments,
    }
}

struct MedianMode<const N: usize>;

trait MedianSampling {
    fn sample(position: Point2<Pixel, u32>, _direction: Direction, image: &YCbCr422Image) -> u8;
}

impl MedianSampling for MedianMode<0> {
    fn sample(position: Point2<Pixel, u32>, _direction: Direction, image: &YCbCr422Image) -> u8 {
        image.at_point(position).y
    }
}

impl MedianSampling for MedianMode<3> {
    fn sample(position: Point2<Pixel, u32>, direction: Direction, image: &YCbCr422Image) -> u8 {
        let offset: Vector2<Pixel, u32> = match direction {
            Direction::Horizontal => Vector2::y_axis(),
            Direction::Vertical => Vector2::x_axis(),
        };

        [
            image.at_point(position - offset).y,
            image.at_point(position).y,
            image.at_point(position + offset).y,
        ]
        .median()
    }
}

impl MedianSampling for MedianMode<5> {
    fn sample(position: Point2<Pixel, u32>, direction: Direction, image: &YCbCr422Image) -> u8 {
        let offset: Vector2<Pixel, u32> = match direction {
            Direction::Horizontal => Vector2::y_axis(),
            Direction::Vertical => Vector2::x_axis(),
        };

        [
            image.at_point(position - offset * 2).y,
            image.at_point(position - offset).y,
            image.at_point(position).y,
            image.at_point(position + offset).y,
            image.at_point(position + offset * 2).y,
        ]
        .median()
    }
}

fn detect_field_color_in_segment(
    segment: &Segment,
    position: u32,
    direction: Direction,
    image: &YCbCr422Image,
) -> Intensity {
    const RADIUS: u32 = 28;

    let color = segment.color;
    let rgb = Rgb::from(color);
    let g_chromaticity = rgb.green_chromaticity();
    let center: Point2<Pixel, u32> = match direction {
        Direction::Horizontal => point![segment.center() as u32, position],
        Direction::Vertical => point![position, segment.center() as u32],
    };

    let right = image.at((center.x() + RADIUS).min(image.width() - 1), center.y());
    let top = image.at(center.x(), center.y().saturating_sub(RADIUS));
    let left = image.at(center.x().saturating_sub(RADIUS), center.y());
    let bottom = image.at(center.x(), (center.y() + RADIUS).min(image.height() - 1));

    let features = Features {
        center: g_chromaticity,
        right: Rgb::from(right).green_chromaticity(),
        top: Rgb::from(top).green_chromaticity(),
        left: Rgb::from(left).green_chromaticity(),
        bottom: Rgb::from(bottom).green_chromaticity(),
    };

    let probability = field_color_tree::predict(&features);
    if probability >= 0.5 {
        Intensity::High
    } else {
        Intensity::Low
    }
}

#[derive(Default)]
struct YCbCr444Sum {
    // 640 pixels each with range 256 requires a 18 bit integer
    y: u32,
    cb: u32,
    cr: u32,
    number_of_summands: u32,
}

impl Add<YCbCr444> for YCbCr444Sum {
    type Output = YCbCr444Sum;

    fn add(self, other: YCbCr444) -> Self::Output {
        Self {
            y: self.y + other.y as u32,
            cb: self.cb + other.cb as u32,
            cr: self.cr + other.cr as u32,
            number_of_summands: self.number_of_summands + 1,
        }
    }
}

impl YCbCr444Sum {
    fn average(&self) -> YCbCr444 {
        YCbCr444 {
            y: (self.y / self.number_of_summands) as u8,
            cb: (self.cb / self.number_of_summands) as u8,
            cr: (self.cr / self.number_of_summands) as u8,
        }
    }
}

fn segment_is_below_limbs(
    scan_line_position: u16,
    segment: &Segment,
    projected_limbs: &[Limb],
) -> bool {
    let projected_y_on_limb = project_onto_limbs(
        point![scan_line_position as f32, segment.end as f32],
        projected_limbs,
    );
    projected_y_on_limb.is_some_and(|projected_y_on_limb| segment.end as f32 > projected_y_on_limb)
}

fn fix_previous_edge_type(segments: &mut [Segment]) {
    if let Some(previous_segment) = segments.last_mut() {
        previous_segment.end_edge_type = EdgeType::LimbBorder;
    }
}

fn average_color_in_segment(
    segment: &Segment,
    position: u32,
    direction: Direction,
    image: &YCbCr422Image,
) -> YCbCr444 {
    let center = match direction {
        Direction::Horizontal => point![segment.center() as u32, position],
        Direction::Vertical => point![position, segment.center() as u32],
    };
    let start = match direction {
        Direction::Horizontal => point![(segment.start + segment.length() / 3) as u32, position],
        Direction::Vertical => point![position, (segment.start + segment.length() / 3) as u32],
    };
    let end = match direction {
        Direction::Horizontal => point![(segment.end - segment.length() / 3 - 1) as u32, position],
        Direction::Vertical => point![position, (segment.end - segment.length() / 3 - 1) as u32],
    };
    let sum = YCbCr444Sum::default()
        + image.at_point(start)
        + image.at_point(center)
        + image.at_point(end);
    sum.average()
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
        state.maximum_difference_position = position;
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
    use types::color::YCbCr422;

    use super::*;

    #[test]
    fn maximum_with_sign_switch() {
        let image = YCbCr422Image::load_from_444_png(
            "../../tests/data/white_wall_with_a_little_desk_in_front.png",
        )
        .unwrap();
        let vertical_stride = 2;
        let vertical_edge_threshold = 16;
        let horizon_y_minimum = 0.0;
        let scan_line = new_vertical_scan_line::<MedianMode<0>>(
            &image,
            12,
            vertical_stride,
            vertical_edge_threshold,
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
        let scan_line = new_vertical_scan_line::<MedianMode<0>>(&image, 0, 2, 1, 0.0, &[]);
        assert_eq!(scan_line.position, 0);
        assert_eq!(scan_line.segments.len(), 1);
        assert_eq!(scan_line.segments[0].start, 0);
        assert_eq!(scan_line.segments[0].end, 3);
    }

    #[test]
    fn image_with_one_vertical_segment_with_median() {
        let image = YCbCr422Image::zero(6, 3);
        let scan_line = new_vertical_scan_line::<MedianMode<3>>(&image, 1, 2, 1, 0.0, &[]);
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

        let scan_line = new_vertical_scan_line::<MedianMode<0>>(&image, 0, 2, 1, 0.0, &[]);
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

        let scan_line = new_vertical_scan_line::<MedianMode<0>>(&image, 0, 2, 1, 0.0, &[]);
        assert_eq!(scan_line.position, 0);
        assert_eq!(scan_line.segments.len(), 1);
        assert_eq!(scan_line.segments[0].color.y, 0);
        assert_eq!(scan_line.segments[0].color.cb, 7);
        assert_eq!(scan_line.segments[0].color.cr, 7);
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

        // y  diff  prev_diff  prev_diff >= thres  diff < thres  prev_diff >= thres && diff < thres
        // 0  0     0          0                   1             0
        // 1  1     0          0                   0             0
        // 1  0     1          1                   1             1 -> end segment at position 2
        // 2  1     0          0                   0             0
        // 2  0     1          1                   1             1 -> end segment at position 6
        // 3  1     0          0                   0             0
        // -> end segment at position 12

        let scan_line = new_vertical_scan_line::<MedianMode<0>>(&image, 0, 2, 1, 0.0, &[]);
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

        let scan_line = new_vertical_scan_line::<MedianMode<3>>(&image, 1, 2, 1, 0.0, &[]);
        assert_eq!(scan_line.position, 1);
        assert_eq!(scan_line.segments.len(), 3);
        assert_eq!(scan_line.segments[0].start, 0);
        assert_eq!(scan_line.segments[0].end, 2);
        assert_eq!(scan_line.segments[0].start_edge_type, EdgeType::ImageBorder);
        assert_eq!(scan_line.segments[0].end_edge_type, EdgeType::Rising);
        assert_eq!(scan_line.segments[1].start, 2);
        assert_eq!(scan_line.segments[1].end, 6);
        assert_eq!(scan_line.segments[1].start_edge_type, EdgeType::Rising);
        assert_eq!(scan_line.segments[1].end_edge_type, EdgeType::Rising);
        assert_eq!(scan_line.segments[2].start, 6);
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

        // y  diff  prev_diff  prev_diff <= -thres  diff > -thres  prev_diff <= -thres && diff > -thres
        // 3   0     0         0                    1              0
        // 2  -1     0         0                    0              0
        // 2   0    -1         1                    1              1 -> end segment at position 2
        // 1  -1     0         0                    0              0
        // 1   0    -1         1                    1              1 -> end segment at position 6
        // 0  -1     0         0                    0              0
        // -> end segment at position 12

        let scan_line = new_vertical_scan_line::<MedianMode<0>>(&image, 0, 2, 1, 0.0, &[]);
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

        let scan_line = new_vertical_scan_line::<MedianMode<3>>(&image, 1, 2, 1, 0.0, &[]);
        assert_eq!(scan_line.position, 1);
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

        let scan_line = new_vertical_scan_line::<MedianMode<0>>(&image, 0, 2, 1, 0.0, &[]);
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

        let scan_line = new_vertical_scan_line::<MedianMode<3>>(&image, 1, 2, 1, 0.0, &[]);
        assert_eq!(scan_line.position, 1);
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

        let scan_line = new_vertical_scan_line::<MedianMode<0>>(&image, 0, 2, 1, 0.0, &[]);
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

        let scan_line = new_vertical_scan_line::<MedianMode<3>>(&image, 1, 2, 1, 0.0, &[]);
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
        assert_eq!([0, 0, 0].median(), 0);
        // first < second == third
        assert_eq!([0, 1, 1].median(), 1);
        // first == second < third
        assert_eq!([0, 0, 1].median(), 0);
        // first == third < second
        assert_eq!([0, 1, 0].median(), 0);
    }

    #[test]
    fn median_of_three_with_different_values() {
        // first <= second <= third
        assert_eq!([0, 1, 2].median(), 1);
        // first <= third < second
        assert_eq!([0, 2, 1].median(), 1);
        // third < first <= second
        assert_eq!([1, 2, 0].median(), 1);
        // second < first <= third
        assert_eq!([1, 0, 2].median(), 1);
        // second <= third < first
        assert_eq!([2, 0, 1].median(), 1);
        // third < second <= first
        assert_eq!([2, 1, 0].median(), 1);
    }

    #[test]
    fn median_of_five_calculates_median() {
        for (first, second, third, fourth, fifth) in iproduct!(0..5, 0..5, 0..5, 0..5, 0..5) {
            let calculated_median = [first, second, third, fourth, fifth].median();
            let mut numbers = [first, second, third, fourth, fifth];
            numbers.sort();
            let real_median = numbers[2];
            assert_eq!(calculated_median,real_median, "test_case: {first} {second} {third} {fourth} {fifth}, median_of_five: {calculated_median}");
        }
    }
}
