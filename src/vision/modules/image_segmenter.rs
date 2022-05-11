use macros::{module, require_some};
use nalgebra::point;

use crate::types::{
    CameraMatrix, EdgeType, FieldColor, ImageSegments, Intensity, ScanGrid, ScanLine, Segment,
    YCbCr444,
};

#[derive(Default)]
pub struct ImageSegmenter;

#[module(vision)]
#[input(path = camera_matrix, data_type = CameraMatrix)]
#[input(path = field_color, data_type = FieldColor)]
#[parameter(path = $this_cycler.image_segmenter.horizontal_edge_threshold, data_type = i16)]
#[parameter(path = $this_cycler.image_segmenter.vertical_edge_threshold, data_type = i16)]
#[parameter(path = $this_cycler.image_segmenter.use_vertical_median, data_type = bool)]
#[main_output(data_type = ImageSegments)]
impl ImageSegmenter {}

impl ImageSegmenter {
    fn cycle(&mut self, context: CycleContext) -> anyhow::Result<MainOutputs> {
        let field_color = require_some!(context.field_color);

        let horizontal_stride = 2;
        let vertical_stride = 2;
        let vertical_stride_in_robot_coordinates = 0.02;
        let scan_grid = new_grid(
            context.image,
            context.camera_matrix,
            field_color,
            horizontal_stride,
            *context.horizontal_edge_threshold,
            vertical_stride,
            vertical_stride_in_robot_coordinates,
            *context.vertical_edge_threshold,
            *context.use_vertical_median,
        );
        Ok(MainOutputs {
            image_segments: Some(ImageSegments { scan_grid }),
        })
    }
}

#[allow(clippy::too_many_arguments)]
fn new_grid(
    image: &Image422,
    camera_matrix: &Option<CameraMatrix>,
    field_color: &FieldColor,
    horizontal_stride: usize,
    horizontal_edge_threshold: i16,
    vertical_stride: usize,
    vertical_stride_in_robot_coordinates: f32,
    vertical_edge_threshold: i16,
    vertical_use_median: bool,
) -> ScanGrid {
    let horizon_y_minimum = camera_matrix.as_ref().map_or(0.0, |camera_matrix| {
        camera_matrix
            .horizon
            .horizon_y_minimum()
            .clamp(0.0, image.height() as f32)
    });

    let mut horizontal_scan_lines = vec![];
    // do not start at horizon because of numerically unstable math
    let mut y = horizon_y_minimum + 1.0;
    while y < image.height() as f32 {
        horizontal_scan_lines.push(new_horizontal_scan_line(
            image,
            field_color,
            y as usize,
            horizontal_stride,
            horizontal_edge_threshold,
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
        vertical_scan_lines: (0..image.width())
            .step_by(horizontal_stride)
            .map(|x_422| {
                new_vertical_scan_line(
                    image,
                    field_color,
                    x_422,
                    vertical_stride,
                    vertical_edge_threshold,
                    vertical_use_median,
                    horizon_y_minimum,
                )
            })
            .collect(),
    }
}

fn next_y_from_current_y(
    image: &Image422,
    camera_matrix: &Option<CameraMatrix>,
    vertical_stride_in_robot_coordinates: f32,
    y: f32,
) -> Option<f32> {
    let camera_matrix = camera_matrix.as_ref()?;

    let center_point_at_y = point![(image.width() / 2) as f32, y];
    let center_point_in_robot_coordinates =
        camera_matrix.pixel_to_robot(&center_point_at_y).ok()?;

    let x_in_robot_coordinates = center_point_in_robot_coordinates[0];
    let y_in_robot_coordinates = center_point_in_robot_coordinates[1];
    let next_x_in_robot_coordinates = x_in_robot_coordinates - vertical_stride_in_robot_coordinates;

    let next_center_point_in_robot_coordinates =
        point![next_x_in_robot_coordinates, y_in_robot_coordinates];
    let next_point_in_pixel_coordinates = camera_matrix
        .ground_to_pixel(&next_center_point_in_robot_coordinates)
        .ok()?;

    Some(next_point_in_pixel_coordinates[1])
}

struct ScanLineState {
    previous_luminance_value: i16,
    previous_luminance_difference: i16,
    maximum_luminance_difference: i16,
    maximum_luminance_difference_position: u16,
    start_position: u16,
    start_edge_type: EdgeType,
}

impl ScanLineState {
    fn new(previous_luminance_value: i16, start_position: u16, start_edge_type: EdgeType) -> Self {
        Self {
            previous_luminance_value,
            previous_luminance_difference: Default::default(),
            maximum_luminance_difference: Default::default(),
            maximum_luminance_difference_position: Default::default(),
            start_position,
            start_edge_type,
        }
    }
}

fn median_of_three(first: u8, second: u8, third: u8) -> u8 {
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

fn new_horizontal_scan_line(
    image: &Image422,
    field_color: &FieldColor,
    position: usize,
    stride: usize,
    edge_threshold: i16,
) -> ScanLine {
    let y = position;

    let first_pixel = image[(0, y)];
    let mut state = ScanLineState::new(first_pixel.y1 as i16, 0, EdgeType::Border);

    let mut segments = vec![];
    for x_422 in (0..image.width()).step_by(stride) {
        let pixel = image[(x_422, y)];

        let luminance_value = pixel.y1 as i16;
        if let Some(segment) =
            detect_edge(&mut state, x_422 as u16, luminance_value, edge_threshold)
        {
            segments.push(set_color_in_horizontal_segment(
                segment,
                image,
                y,
                field_color,
            ));
        }
    }

    let last_segment = Segment {
        start: state.start_position,
        end: image.width() as u16,
        start_edge_type: state.start_edge_type,
        end_edge_type: EdgeType::Border,
        color: Default::default(),
        field_color: Intensity::Low,
    };
    segments.push(set_color_in_horizontal_segment(
        last_segment,
        image,
        y,
        field_color,
    ));

    ScanLine {
        position: position as u16,
        segments,
    }
}

fn new_vertical_scan_line(
    image: &Image422,
    field_color: &FieldColor,
    position_422: usize,
    stride: usize,
    edge_threshold: i16,
    use_median: bool,
    horizon_y_minimum: f32,
) -> ScanLine {
    let x_422 = position_422;
    let (start_y, end_y) = if use_median {
        ((horizon_y_minimum as usize) + 1, image.height() - 1)
    } else {
        (horizon_y_minimum as usize, image.height())
    };
    if start_y + 1 >= image.height() {
        return ScanLine {
            position: position_422 as u16,
            segments: Vec::new(),
        };
    }

    let first_pixel = image[(x_422, start_y)];
    let luminance_value_of_first_pixel = if use_median {
        let previous_pixel = image[(x_422, start_y - 1)];
        let next_pixel = image[(x_422, start_y + 1)];
        median_of_three(previous_pixel.y1, first_pixel.y1, next_pixel.y1)
    } else {
        first_pixel.y1
    } as i16;
    let mut state = ScanLineState::new(
        luminance_value_of_first_pixel,
        horizon_y_minimum as u16,
        EdgeType::Border,
    );

    let mut segments = vec![];
    for y in (start_y..end_y).step_by(stride) {
        let pixel = image[(x_422, y)];

        let luminance_value = if use_median {
            let previous_pixel = image[(x_422, y - 1)];
            let next_pixel = image[(x_422, y + 1)];
            median_of_three(previous_pixel.y1, pixel.y1, next_pixel.y1)
        } else {
            pixel.y1
        } as i16;

        if let Some(segment) = detect_edge(&mut state, y as u16, luminance_value, edge_threshold) {
            segments.push(set_color_in_vertical_segment(
                segment,
                image,
                x_422,
                field_color,
            ));
        }
    }

    let last_segment = Segment {
        start: state.start_position,
        end: image.height() as u16,
        start_edge_type: state.start_edge_type,
        end_edge_type: EdgeType::Border,
        color: Default::default(),
        field_color: Intensity::Low,
    };
    segments.push(set_color_in_vertical_segment(
        last_segment,
        image,
        x_422,
        field_color,
    ));

    ScanLine {
        position: position_422 as u16,
        segments,
    }
}

fn set_color_in_vertical_segment(
    mut segment: Segment,
    image: &Image422,
    x_422: usize,
    field_color: &FieldColor,
) -> Segment {
    segment.color = if segment.length() >= 4 {
        let spacing = segment.length() / 4;
        let first_position = segment.start + spacing;
        let second_position = segment.start + 2 * spacing;
        let third_position = segment.start + 3 * spacing;

        let first_pixel = image[(x_422, first_position as usize)];
        let second_pixel = image[(x_422, second_position as usize)];
        let third_pixel = image[(x_422, third_position as usize)];

        let y = median_of_three(first_pixel.y1, second_pixel.y1, third_pixel.y1);
        let cb = median_of_three(first_pixel.cb, second_pixel.cb, third_pixel.cb);
        let cr = median_of_three(first_pixel.cr, second_pixel.cr, third_pixel.cr);
        YCbCr444::new(y, cb, cr)
    } else {
        let position = segment.start + segment.length() / 2;
        image[(x_422, position as usize)].into()
    };
    segment.field_color = field_color.get_intensity(segment.color);
    segment
}

fn set_color_in_horizontal_segment(
    mut segment: Segment,
    image: &Image422,
    y: usize,
    field_color: &FieldColor,
) -> Segment {
    segment.color = if segment.length() >= 4 {
        let spacing = segment.length() / 4;
        let first_position = segment.start + spacing;
        let second_position = segment.start + 2 * spacing;
        let third_position = segment.start + 3 * spacing;

        let first_pixel = image[(first_position as usize, y)];
        let second_pixel = image[(second_position as usize, y)];
        let third_pixel = image[(third_position as usize, y)];

        let y = median_of_three(first_pixel.y1, second_pixel.y1, third_pixel.y1);
        let cb = median_of_three(first_pixel.cb, second_pixel.cb, third_pixel.cb);
        let cr = median_of_three(first_pixel.cr, second_pixel.cr, third_pixel.cr);
        YCbCr444::new(y, cb, cr)
    } else {
        let position = segment.start + segment.length() / 2;
        image[(position as usize, y)].into()
    };
    segment.field_color = field_color.get_intensity(segment.color);
    segment
}

fn detect_edge(
    state: &mut ScanLineState,
    position: u16,
    luminance_value: i16,
    edge_threshold: i16,
) -> Option<Segment> {
    let luminance_difference = luminance_value - state.previous_luminance_value;

    let differences_have_initial_values =
        state.maximum_luminance_difference == 0 && luminance_difference == 0;
    let new_difference_is_more_positive = state.maximum_luminance_difference >= 0
        && luminance_difference >= state.maximum_luminance_difference;
    let new_difference_is_more_negative = state.maximum_luminance_difference <= 0
        && luminance_difference <= state.maximum_luminance_difference;

    if luminance_difference.abs() >= edge_threshold
        && (differences_have_initial_values
            || new_difference_is_more_positive
            || new_difference_is_more_negative)
    {
        state.maximum_luminance_difference = luminance_difference;
        state.maximum_luminance_difference_position = position;
    }

    let found_rising_edge = state.previous_luminance_difference >= edge_threshold
        && luminance_difference < edge_threshold;
    let found_falling_edge = state.previous_luminance_difference <= -edge_threshold
        && luminance_difference > -edge_threshold;

    let segment = if found_rising_edge || found_falling_edge {
        let end_edge_type = if found_rising_edge {
            EdgeType::Rising
        } else {
            EdgeType::Falling
        };
        let segment = Segment {
            start: state.start_position,
            end: state.maximum_luminance_difference_position,
            start_edge_type: state.start_edge_type,
            end_edge_type,
            color: Default::default(),
            field_color: Intensity::Low,
        };
        state.maximum_luminance_difference = 0;
        state.start_position = state.maximum_luminance_difference_position;
        state.start_edge_type = end_edge_type;

        Some(segment)
    } else {
        None
    };

    state.previous_luminance_value = luminance_value;
    state.previous_luminance_difference = luminance_difference;

    segment
}

#[cfg(test)]
mod tests {
    use crate::types::YCbCr422;

    use super::*;

    #[test]
    fn maximum_with_sign_switch() {
        let image = Image422::load_from_ycbcr_444_file(
            "tests/data/white_wall_with_a_little_desk_in_front.png",
        )
        .unwrap();
        let field_color = FieldColor {
            red_chromaticity_threshold: 0.37,
            blue_chromaticity_threshold: 0.38,
            lower_green_chromaticity_threshold: 0.4,
            upper_green_chromaticity_threshold: 0.43,
        };
        let vertical_stride = 2;
        let vertical_edge_threshold = 16;
        let vertical_use_median = false;
        let horizon_y_minimum = 0.0;
        let scan_line = new_vertical_scan_line(
            &image,
            &field_color,
            12,
            vertical_stride,
            vertical_edge_threshold,
            vertical_use_median,
            horizon_y_minimum,
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
    fn image_with_one_horizontal_segment() {
        let image = Image422::zero(3, 3);
        let field_color = FieldColor {
            red_chromaticity_threshold: 0.37,
            blue_chromaticity_threshold: 0.38,
            lower_green_chromaticity_threshold: 0.4,
            upper_green_chromaticity_threshold: 0.43,
        };
        let scan_line = new_horizontal_scan_line(&image, &field_color, 0, 2, 1);
        assert_eq!(scan_line.position, 0);
        assert_eq!(scan_line.segments.len(), 1);
        assert_eq!(scan_line.segments[0].start, 0);
        assert_eq!(scan_line.segments[0].end, 3);
    }

    #[test]
    fn image_with_one_vertical_segment_without_median() {
        let image = Image422::zero(3, 3);
        let field_color = FieldColor {
            red_chromaticity_threshold: 0.37,
            blue_chromaticity_threshold: 0.38,
            lower_green_chromaticity_threshold: 0.4,
            upper_green_chromaticity_threshold: 0.43,
        };
        let scan_line = new_vertical_scan_line(&image, &field_color, 0, 2, 1, false, 0.0);
        assert_eq!(scan_line.position, 0);
        assert_eq!(scan_line.segments.len(), 1);
        assert_eq!(scan_line.segments[0].start, 0);
        assert_eq!(scan_line.segments[0].end, 3);
    }

    #[test]
    fn image_with_one_vertical_segment_with_median() {
        let image = Image422::zero(3, 3);
        let field_color = FieldColor {
            red_chromaticity_threshold: 0.37,
            blue_chromaticity_threshold: 0.38,
            lower_green_chromaticity_threshold: 0.4,
            upper_green_chromaticity_threshold: 0.43,
        };
        let scan_line = new_vertical_scan_line(&image, &field_color, 0, 2, 1, true, 0.0);
        assert_eq!(scan_line.position, 0);
        assert_eq!(scan_line.segments.len(), 1);
        assert_eq!(scan_line.segments[0].start, 0);
        assert_eq!(scan_line.segments[0].end, 3);
    }

    #[test]
    fn image_vertical_color_three_pixels() {
        let mut image = Image422::zero(1, 3);
        let field_color = FieldColor {
            red_chromaticity_threshold: 0.37,
            blue_chromaticity_threshold: 0.38,
            lower_green_chromaticity_threshold: 0.4,
            upper_green_chromaticity_threshold: 0.43,
        };
        // only evaluating every second 422 pixel
        image[(0, 0)] = YCbCr422::new(0, 10, 10, 10);
        image[(0, 1)] = YCbCr422::new(0, 15, 14, 13);
        image[(0, 2)] = YCbCr422::new(0, 10, 10, 10);

        let scan_line = new_vertical_scan_line(&image, &field_color, 0, 2, 1, false, 0.0);
        assert_eq!(scan_line.position, 0);
        assert_eq!(scan_line.segments.len(), 1);
        assert_eq!(scan_line.segments[0].color.y, 0);
        assert_eq!(scan_line.segments[0].color.cb, 15);
        assert_eq!(scan_line.segments[0].color.cr, 13);
    }

    #[test]
    fn image_vertical_color_twelve_pixels() {
        let mut image = Image422::zero(1, 12);
        let field_color = FieldColor {
            red_chromaticity_threshold: 0.37,
            blue_chromaticity_threshold: 0.38,
            lower_green_chromaticity_threshold: 0.4,
            upper_green_chromaticity_threshold: 0.43,
        };
        // only evaluating every second 422 pixel
        image[(0, 0)] = YCbCr422::new(0, 10, 10, 10);
        image[(0, 1)] = YCbCr422::new(0, 10, 10, 10);
        image[(0, 2)] = YCbCr422::new(0, 10, 10, 10);
        image[(0, 3)] = YCbCr422::new(0, 1, 1, 1);
        image[(0, 4)] = YCbCr422::new(0, 10, 10, 10);
        image[(0, 5)] = YCbCr422::new(0, 10, 10, 10);
        image[(0, 6)] = YCbCr422::new(0, 1, 1, 1);
        image[(0, 7)] = YCbCr422::new(0, 10, 10, 10);
        image[(0, 8)] = YCbCr422::new(0, 10, 10, 10);
        image[(0, 9)] = YCbCr422::new(0, 2, 2, 2);
        image[(0, 10)] = YCbCr422::new(0, 10, 10, 10);
        image[(0, 11)] = YCbCr422::new(0, 10, 10, 10);

        let scan_line = new_vertical_scan_line(&image, &field_color, 0, 2, 1, false, 0.0);
        assert_eq!(scan_line.position, 0);
        assert_eq!(scan_line.segments.len(), 1);
        assert_eq!(scan_line.segments[0].color.y, 0);
        assert_eq!(scan_line.segments[0].color.cb, 1);
        assert_eq!(scan_line.segments[0].color.cr, 1);
    }

    #[test]
    fn image_horizontal_color_three_pixels() {
        let mut image = Image422::zero(3, 1);
        let field_color = FieldColor {
            red_chromaticity_threshold: 0.37,
            blue_chromaticity_threshold: 0.38,
            lower_green_chromaticity_threshold: 0.4,
            upper_green_chromaticity_threshold: 0.43,
        };
        // only evaluating every second 422 pixel
        image[(0, 0)] = YCbCr422::new(0, 10, 10, 10);
        image[(1, 0)] = YCbCr422::new(0, 15, 14, 13);
        image[(2, 0)] = YCbCr422::new(0, 10, 10, 10);

        let scan_line = new_horizontal_scan_line(&image, &field_color, 0, 2, 1);
        assert_eq!(scan_line.position, 0);
        assert_eq!(scan_line.segments.len(), 1);
        assert_eq!(scan_line.segments[0].color.y, 0);
        assert_eq!(scan_line.segments[0].color.cb, 15);
        assert_eq!(scan_line.segments[0].color.cr, 13);
    }

    #[test]
    fn image_horizontal_color_twelve_pixels() {
        let mut image = Image422::zero(12, 1);
        let field_color = FieldColor {
            red_chromaticity_threshold: 0.37,
            blue_chromaticity_threshold: 0.38,
            lower_green_chromaticity_threshold: 0.4,
            upper_green_chromaticity_threshold: 0.43,
        };
        // only evaluating every second 422 pixel
        image[(0, 0)] = YCbCr422::new(0, 10, 10, 10);
        image[(1, 0)] = YCbCr422::new(0, 10, 10, 10);
        image[(2, 0)] = YCbCr422::new(0, 10, 10, 10);
        image[(3, 0)] = YCbCr422::new(0, 1, 1, 1);
        image[(4, 0)] = YCbCr422::new(0, 10, 10, 10);
        image[(5, 0)] = YCbCr422::new(0, 10, 10, 10);
        image[(6, 0)] = YCbCr422::new(0, 1, 1, 1);
        image[(7, 0)] = YCbCr422::new(0, 10, 10, 10);
        image[(8, 0)] = YCbCr422::new(0, 10, 10, 10);
        image[(9, 0)] = YCbCr422::new(0, 2, 2, 2);
        image[(10, 0)] = YCbCr422::new(0, 10, 10, 10);
        image[(11, 0)] = YCbCr422::new(0, 10, 10, 10);

        let scan_line = new_horizontal_scan_line(&image, &field_color, 0, 2, 1);
        assert_eq!(scan_line.position, 0);
        assert_eq!(scan_line.segments.len(), 1);
        assert_eq!(scan_line.segments[0].color.y, 0);
        assert_eq!(scan_line.segments[0].color.cb, 1);
        assert_eq!(scan_line.segments[0].color.cr, 1);
    }

    #[test]
    fn image_with_three_horizontal_increasing_segments() {
        let mut image = Image422::zero(12, 3);
        let field_color = FieldColor {
            red_chromaticity_threshold: 0.37,
            blue_chromaticity_threshold: 0.38,
            lower_green_chromaticity_threshold: 0.4,
            upper_green_chromaticity_threshold: 0.43,
        };
        // only evaluating every secondth 422 pixel
        image[(0, 0)] = YCbCr422::new(0, 0, 0, 0);
        image[(1, 0)] = YCbCr422::new(0, 0, 0, 0); // skipped

        // segment boundary will be here

        image[(2, 0)] = YCbCr422::new(1, 0, 0, 0);
        image[(3, 0)] = YCbCr422::new(1, 0, 0, 0); // skipped
        image[(4, 0)] = YCbCr422::new(1, 0, 0, 0);
        image[(5, 0)] = YCbCr422::new(1, 0, 0, 0); // skipped

        // segment boundary will be here

        image[(6, 0)] = YCbCr422::new(2, 0, 0, 0);
        image[(7, 0)] = YCbCr422::new(2, 0, 0, 0); // skipped
        image[(8, 0)] = YCbCr422::new(2, 0, 0, 0);
        image[(9, 0)] = YCbCr422::new(2, 0, 0, 0); // skipped
        image[(10, 0)] = YCbCr422::new(3, 0, 0, 0);
        image[(11, 0)] = YCbCr422::new(3, 0, 0, 0); // skipped

        // segment boundary will be here

        // y  diff  prev_diff  prev_diff >= thres  diff < thres  prev_diff >= thres && diff < thres
        // 0  0     0          0                   1             0
        // 1  1     0          0                   0             0
        // 1  0     1          1                   1             1 -> end segment at position 2
        // 2  1     0          0                   0             0
        // 2  0     1          1                   1             1 -> end segment at position 6
        // 3  1     0          0                   0             0
        // -> end segment at position 12

        let scan_line = new_horizontal_scan_line(&image, &field_color, 0, 2, 1);
        assert_eq!(scan_line.position, 0);
        assert_eq!(scan_line.segments.len(), 3);
        assert_eq!(scan_line.segments[0].start, 0);
        assert_eq!(scan_line.segments[0].end, 2);
        assert_eq!(scan_line.segments[0].start_edge_type, EdgeType::Border);
        assert_eq!(scan_line.segments[0].end_edge_type, EdgeType::Rising);
        assert_eq!(scan_line.segments[1].start, 2);
        assert_eq!(scan_line.segments[1].end, 6);
        assert_eq!(scan_line.segments[1].start_edge_type, EdgeType::Rising);
        assert_eq!(scan_line.segments[1].end_edge_type, EdgeType::Rising);
        assert_eq!(scan_line.segments[2].start, 6);
        assert_eq!(scan_line.segments[2].end, 12);
        assert_eq!(scan_line.segments[2].start_edge_type, EdgeType::Rising);
        assert_eq!(scan_line.segments[2].end_edge_type, EdgeType::Border);
    }

    #[test]
    fn image_with_three_vertical_increasing_segments_without_median() {
        let mut image = Image422::zero(3, 12);
        let field_color = FieldColor {
            red_chromaticity_threshold: 0.37,
            blue_chromaticity_threshold: 0.38,
            lower_green_chromaticity_threshold: 0.4,
            upper_green_chromaticity_threshold: 0.43,
        };
        // only evaluating every secondth pixel
        image[(0, 0)] = YCbCr422::new(0, 0, 0, 0);
        image[(0, 1)] = YCbCr422::new(0, 0, 0, 0); // skipped

        // segment boundary will be here

        image[(0, 2)] = YCbCr422::new(1, 0, 0, 0);
        image[(0, 3)] = YCbCr422::new(1, 0, 0, 0); // skipped
        image[(0, 4)] = YCbCr422::new(1, 0, 0, 0);
        image[(0, 5)] = YCbCr422::new(1, 0, 0, 0); // skipped

        // segment boundary will be here

        image[(0, 6)] = YCbCr422::new(2, 0, 0, 0);
        image[(0, 7)] = YCbCr422::new(2, 0, 0, 0); // skipped
        image[(0, 8)] = YCbCr422::new(2, 0, 0, 0);
        image[(0, 9)] = YCbCr422::new(2, 0, 0, 0); // skipped
        image[(0, 10)] = YCbCr422::new(3, 0, 0, 0);
        image[(0, 11)] = YCbCr422::new(3, 0, 0, 0); // skipped

        // segment boundary will be here

        // y  diff  prev_diff  prev_diff >= thres  diff < thres  prev_diff >= thres && diff < thres
        // 0  0     0          0                   1             0
        // 1  1     0          0                   0             0
        // 1  0     1          1                   1             1 -> end segment at position 2
        // 2  1     0          0                   0             0
        // 2  0     1          1                   1             1 -> end segment at position 6
        // 3  1     0          0                   0             0
        // -> end segment at position 12

        let scan_line = new_vertical_scan_line(&image, &field_color, 0, 2, 1, false, 0.0);
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
        let mut image = Image422::zero(3, 12);
        let field_color = FieldColor {
            red_chromaticity_threshold: 0.37,
            blue_chromaticity_threshold: 0.38,
            lower_green_chromaticity_threshold: 0.4,
            upper_green_chromaticity_threshold: 0.43,
        };
        // only evaluating every secondth pixel
        image[(0, 0)] = YCbCr422::new(0, 0, 0, 0);
        image[(0, 1)] = YCbCr422::new(0, 0, 0, 0); // skipped
        image[(0, 2)] = YCbCr422::new(1, 0, 0, 0);

        // segment boundary will be here

        image[(0, 3)] = YCbCr422::new(1, 0, 0, 0); // skipped
        image[(0, 4)] = YCbCr422::new(1, 0, 0, 0);
        image[(0, 5)] = YCbCr422::new(1, 0, 0, 0); // skipped
        image[(0, 6)] = YCbCr422::new(2, 0, 0, 0);

        // segment boundary will be here

        image[(0, 7)] = YCbCr422::new(2, 0, 0, 0); // skipped
        image[(0, 8)] = YCbCr422::new(2, 0, 0, 0);
        image[(0, 9)] = YCbCr422::new(2, 0, 0, 0); // skipped
        image[(0, 10)] = YCbCr422::new(3, 0, 0, 0);
        image[(0, 11)] = YCbCr422::new(3, 0, 0, 0); // skipped

        // segment boundary will be here

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

        let scan_line = new_vertical_scan_line(&image, &field_color, 0, 2, 1, true, 0.0);
        assert_eq!(scan_line.position, 0);
        assert_eq!(scan_line.segments.len(), 3);
        assert_eq!(scan_line.segments[0].start, 0);
        assert_eq!(scan_line.segments[0].end, 3);
        assert_eq!(scan_line.segments[0].start_edge_type, EdgeType::Border);
        assert_eq!(scan_line.segments[0].end_edge_type, EdgeType::Rising);
        assert_eq!(scan_line.segments[1].start, 3);
        assert_eq!(scan_line.segments[1].end, 7);
        assert_eq!(scan_line.segments[1].start_edge_type, EdgeType::Rising);
        assert_eq!(scan_line.segments[1].end_edge_type, EdgeType::Rising);
        assert_eq!(scan_line.segments[2].start, 7);
        assert_eq!(scan_line.segments[2].end, 12);
        assert_eq!(scan_line.segments[2].start_edge_type, EdgeType::Rising);
        assert_eq!(scan_line.segments[2].end_edge_type, EdgeType::Border);
    }

    #[test]
    fn image_with_three_horizontal_decreasing_segments() {
        let mut image = Image422::zero(12, 3);
        let field_color = FieldColor {
            red_chromaticity_threshold: 0.37,
            blue_chromaticity_threshold: 0.38,
            lower_green_chromaticity_threshold: 0.4,
            upper_green_chromaticity_threshold: 0.43,
        };
        // only evaluating every secondth 422 pixel
        image[(0, 0)] = YCbCr422::new(3, 0, 0, 0);
        image[(1, 0)] = YCbCr422::new(3, 0, 0, 0); // skipped

        // segment boundary will be here

        image[(2, 0)] = YCbCr422::new(2, 0, 0, 0);
        image[(3, 0)] = YCbCr422::new(2, 0, 0, 0); // skipped
        image[(4, 0)] = YCbCr422::new(2, 0, 0, 0);
        image[(5, 0)] = YCbCr422::new(2, 0, 0, 0); // skipped

        // segment boundary will be here

        image[(6, 0)] = YCbCr422::new(1, 0, 0, 0);
        image[(7, 0)] = YCbCr422::new(1, 0, 0, 0); // skipped
        image[(8, 0)] = YCbCr422::new(1, 0, 0, 0);
        image[(9, 0)] = YCbCr422::new(1, 0, 0, 0); // skipped
        image[(10, 0)] = YCbCr422::new(0, 0, 0, 0);
        image[(11, 0)] = YCbCr422::new(0, 0, 0, 0); // skipped

        // segment boundary will be here

        // y  diff  prev_diff  prev_diff <= -thres  diff > -thres  prev_diff <= -thres && diff > -thres
        // 3   0     0         0                    1              0
        // 2  -1     0         0                    0              0
        // 2   0    -1         1                    1              1 -> end segment at position 2
        // 1  -1     0         0                    0              0
        // 1   0    -1         1                    1              1 -> end segment at position 6
        // 0  -1     0         0                    0              0
        // -> end segment at position 12

        let scan_line = new_horizontal_scan_line(&image, &field_color, 0, 2, 1);
        assert_eq!(scan_line.position, 0);
        assert_eq!(scan_line.segments.len(), 3);
        assert_eq!(scan_line.segments[0].start, 0);
        assert_eq!(scan_line.segments[0].end, 2);
        assert_eq!(scan_line.segments[0].start_edge_type, EdgeType::Border);
        assert_eq!(scan_line.segments[0].end_edge_type, EdgeType::Falling);
        assert_eq!(scan_line.segments[1].start, 2);
        assert_eq!(scan_line.segments[1].end, 6);
        assert_eq!(scan_line.segments[1].start_edge_type, EdgeType::Falling);
        assert_eq!(scan_line.segments[1].end_edge_type, EdgeType::Falling);
        assert_eq!(scan_line.segments[2].start, 6);
        assert_eq!(scan_line.segments[2].end, 12);
        assert_eq!(scan_line.segments[2].start_edge_type, EdgeType::Falling);
        assert_eq!(scan_line.segments[2].end_edge_type, EdgeType::Border);
    }

    #[test]
    fn image_with_three_vertical_decreasing_segments_without_median() {
        let mut image = Image422::zero(3, 12);
        let field_color = FieldColor {
            red_chromaticity_threshold: 0.37,
            blue_chromaticity_threshold: 0.38,
            lower_green_chromaticity_threshold: 0.4,
            upper_green_chromaticity_threshold: 0.43,
        };
        // only evaluating every secondth 422 pixel
        image[(0, 0)] = YCbCr422::new(3, 0, 0, 0);
        image[(0, 1)] = YCbCr422::new(3, 0, 0, 0); // skipped

        // segment boundary will be here

        image[(0, 2)] = YCbCr422::new(2, 0, 0, 0);
        image[(0, 3)] = YCbCr422::new(2, 0, 0, 0); // skipped
        image[(0, 4)] = YCbCr422::new(2, 0, 0, 0);
        image[(0, 5)] = YCbCr422::new(2, 0, 0, 0); // skipped

        // segment boundary will be here

        image[(0, 6)] = YCbCr422::new(1, 0, 0, 0);
        image[(0, 7)] = YCbCr422::new(1, 0, 0, 0); // skipped
        image[(0, 8)] = YCbCr422::new(1, 0, 0, 0);
        image[(0, 9)] = YCbCr422::new(1, 0, 0, 0); // skipped
        image[(0, 10)] = YCbCr422::new(0, 0, 0, 0);
        image[(0, 11)] = YCbCr422::new(0, 0, 0, 0); // skipped

        // segment boundary will be here

        // y  diff  prev_diff  prev_diff <= -thres  diff > -thres  prev_diff <= -thres && diff > -thres
        // 3   0     0         0                    1              0
        // 2  -1     0         0                    0              0
        // 2   0    -1         1                    1              1 -> end segment at position 2
        // 1  -1     0         0                    0              0
        // 1   0    -1         1                    1              1 -> end segment at position 6
        // 0  -1     0         0                    0              0
        // -> end segment at position 12

        let scan_line = new_vertical_scan_line(&image, &field_color, 0, 2, 1, false, 0.0);
        assert_eq!(scan_line.position, 0);
        assert_eq!(scan_line.segments.len(), 3);
        assert_eq!(scan_line.segments[0].start, 0);
        assert_eq!(scan_line.segments[0].end, 2);
        assert_eq!(scan_line.segments[0].start_edge_type, EdgeType::Border);
        assert_eq!(scan_line.segments[0].end_edge_type, EdgeType::Falling);
        assert_eq!(scan_line.segments[1].start, 2);
        assert_eq!(scan_line.segments[1].end, 6);
        assert_eq!(scan_line.segments[1].start_edge_type, EdgeType::Falling);
        assert_eq!(scan_line.segments[1].end_edge_type, EdgeType::Falling);
        assert_eq!(scan_line.segments[2].start, 6);
        assert_eq!(scan_line.segments[2].end, 12);
        assert_eq!(scan_line.segments[2].start_edge_type, EdgeType::Falling);
        assert_eq!(scan_line.segments[2].end_edge_type, EdgeType::Border);
    }

    #[test]
    fn image_with_three_vertical_decreasing_segments_with_median() {
        let mut image = Image422::zero(3, 12);
        let field_color = FieldColor {
            red_chromaticity_threshold: 0.37,
            blue_chromaticity_threshold: 0.38,
            lower_green_chromaticity_threshold: 0.4,
            upper_green_chromaticity_threshold: 0.43,
        };
        // only evaluating every secondth 422 pixel
        image[(0, 0)] = YCbCr422::new(3, 0, 0, 0);
        image[(0, 1)] = YCbCr422::new(3, 0, 0, 0); // skipped
        image[(0, 2)] = YCbCr422::new(2, 0, 0, 0);

        // segment boundary will be here

        image[(0, 3)] = YCbCr422::new(2, 0, 0, 0); // skipped
        image[(0, 4)] = YCbCr422::new(2, 0, 0, 0);
        image[(0, 5)] = YCbCr422::new(2, 0, 0, 0); // skipped
        image[(0, 6)] = YCbCr422::new(1, 0, 0, 0);

        // segment boundary will be here

        image[(0, 7)] = YCbCr422::new(1, 0, 0, 0); // skipped
        image[(0, 8)] = YCbCr422::new(1, 0, 0, 0);
        image[(0, 9)] = YCbCr422::new(1, 0, 0, 0); // skipped
        image[(0, 10)] = YCbCr422::new(0, 0, 0, 0);
        image[(0, 11)] = YCbCr422::new(0, 0, 0, 0); // skipped

        // segment boundary will be here

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

        let scan_line = new_vertical_scan_line(&image, &field_color, 0, 2, 1, true, 0.0);
        assert_eq!(scan_line.position, 0);
        assert_eq!(scan_line.segments.len(), 3);
        assert_eq!(scan_line.segments[0].start, 0);
        assert_eq!(scan_line.segments[0].end, 3);
        assert_eq!(scan_line.segments[0].start_edge_type, EdgeType::Border);
        assert_eq!(scan_line.segments[0].end_edge_type, EdgeType::Falling);
        assert_eq!(scan_line.segments[1].start, 3);
        assert_eq!(scan_line.segments[1].end, 7);
        assert_eq!(scan_line.segments[1].start_edge_type, EdgeType::Falling);
        assert_eq!(scan_line.segments[1].end_edge_type, EdgeType::Falling);
        assert_eq!(scan_line.segments[2].start, 7);
        assert_eq!(scan_line.segments[2].end, 12);
        assert_eq!(scan_line.segments[2].start_edge_type, EdgeType::Falling);
        assert_eq!(scan_line.segments[2].end_edge_type, EdgeType::Border);
    }

    #[test]
    fn image_with_three_horizontal_segments_with_higher_differences() {
        let mut image = Image422::zero(44, 3);
        let field_color = FieldColor {
            red_chromaticity_threshold: 0.37,
            blue_chromaticity_threshold: 0.38,
            lower_green_chromaticity_threshold: 0.4,
            upper_green_chromaticity_threshold: 0.43,
        };
        // only evaluating every secondth 422 pixel
        image[(0, 0)] = YCbCr422::new(0, 0, 0, 0);
        image[(1, 0)] = YCbCr422::new(0, 0, 0, 0); // skipped
        image[(2, 0)] = YCbCr422::new(0, 0, 0, 0);
        image[(3, 0)] = YCbCr422::new(0, 0, 0, 0); // skipped
        image[(4, 0)] = YCbCr422::new(0, 0, 0, 0);
        image[(5, 0)] = YCbCr422::new(0, 0, 0, 0); // skipped
        image[(6, 0)] = YCbCr422::new(0, 0, 0, 0);
        image[(7, 0)] = YCbCr422::new(0, 0, 0, 0); // skipped
        image[(8, 0)] = YCbCr422::new(1, 0, 0, 0);
        image[(9, 0)] = YCbCr422::new(1, 0, 0, 0); // skipped
        image[(10, 0)] = YCbCr422::new(2, 0, 0, 0);
        image[(11, 0)] = YCbCr422::new(2, 0, 0, 0); // skipped
        image[(12, 0)] = YCbCr422::new(3, 0, 0, 0);
        image[(13, 0)] = YCbCr422::new(3, 0, 0, 0); // skipped
        image[(14, 0)] = YCbCr422::new(4, 0, 0, 0);
        image[(15, 0)] = YCbCr422::new(4, 0, 0, 0); // skipped

        // segment boundary will be here

        image[(16, 0)] = YCbCr422::new(5, 0, 0, 0);
        image[(17, 0)] = YCbCr422::new(5, 0, 0, 0); // skipped
        image[(18, 0)] = YCbCr422::new(4, 0, 0, 0);
        image[(19, 0)] = YCbCr422::new(4, 0, 0, 0); // skipped
        image[(20, 0)] = YCbCr422::new(3, 0, 0, 0);
        image[(21, 0)] = YCbCr422::new(3, 0, 0, 0); // skipped
        image[(22, 0)] = YCbCr422::new(2, 0, 0, 0);
        image[(23, 0)] = YCbCr422::new(2, 0, 0, 0); // skipped
        image[(24, 0)] = YCbCr422::new(1, 0, 0, 0);
        image[(25, 0)] = YCbCr422::new(1, 0, 0, 0); // skipped

        // segment boundary will be here

        image[(26, 0)] = YCbCr422::new(0, 0, 0, 0);
        image[(27, 0)] = YCbCr422::new(0, 0, 0, 0); // skipped
        image[(28, 0)] = YCbCr422::new(0, 0, 0, 0);
        image[(29, 0)] = YCbCr422::new(0, 0, 0, 0); // skipped
        image[(30, 0)] = YCbCr422::new(0, 0, 0, 0);
        image[(31, 0)] = YCbCr422::new(0, 0, 0, 0); // skipped
        image[(32, 0)] = YCbCr422::new(0, 0, 0, 0);
        image[(33, 0)] = YCbCr422::new(0, 0, 0, 0); // skipped
        image[(34, 0)] = YCbCr422::new(0, 0, 0, 0);
        image[(35, 0)] = YCbCr422::new(0, 0, 0, 0); // skipped
        image[(36, 0)] = YCbCr422::new(0, 0, 0, 0);
        image[(37, 0)] = YCbCr422::new(0, 0, 0, 0); // skipped
        image[(38, 0)] = YCbCr422::new(0, 0, 0, 0);
        image[(39, 0)] = YCbCr422::new(0, 0, 0, 0); // skipped
        image[(40, 0)] = YCbCr422::new(0, 0, 0, 0);
        image[(41, 0)] = YCbCr422::new(0, 0, 0, 0); // skipped
        image[(42, 0)] = YCbCr422::new(0, 0, 0, 0);
        image[(43, 0)] = YCbCr422::new(0, 0, 0, 0); // skipped

        // segment boundary will be here

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

        let scan_line = new_horizontal_scan_line(&image, &field_color, 0, 2, 1);
        assert_eq!(scan_line.position, 0);
        assert_eq!(scan_line.segments.len(), 3);
        assert_eq!(scan_line.segments[0].start, 0);
        assert_eq!(scan_line.segments[0].end, 16);
        assert_eq!(scan_line.segments[0].start_edge_type, EdgeType::Border);
        assert_eq!(scan_line.segments[0].end_edge_type, EdgeType::Rising);
        assert_eq!(scan_line.segments[1].start, 16);
        assert_eq!(scan_line.segments[1].end, 26);
        assert_eq!(scan_line.segments[1].start_edge_type, EdgeType::Rising);
        assert_eq!(scan_line.segments[1].end_edge_type, EdgeType::Falling);
        assert_eq!(scan_line.segments[2].start, 26);
        assert_eq!(scan_line.segments[2].end, 44);
        assert_eq!(scan_line.segments[2].start_edge_type, EdgeType::Falling);
        assert_eq!(scan_line.segments[2].end_edge_type, EdgeType::Border);
    }

    #[test]
    fn image_with_three_vertical_segments_with_higher_differences_without_median() {
        let mut image = Image422::zero(3, 44);
        let field_color = FieldColor {
            red_chromaticity_threshold: 0.37,
            blue_chromaticity_threshold: 0.38,
            lower_green_chromaticity_threshold: 0.4,
            upper_green_chromaticity_threshold: 0.43,
        };
        // only evaluating every secondth 422 pixel
        image[(0, 0)] = YCbCr422::new(0, 0, 0, 0);
        image[(0, 1)] = YCbCr422::new(0, 0, 0, 0); // skipped
        image[(0, 2)] = YCbCr422::new(0, 0, 0, 0);
        image[(0, 3)] = YCbCr422::new(0, 0, 0, 0); // skipped
        image[(0, 4)] = YCbCr422::new(0, 0, 0, 0);
        image[(0, 5)] = YCbCr422::new(0, 0, 0, 0); // skipped
        image[(0, 6)] = YCbCr422::new(0, 0, 0, 0);
        image[(0, 7)] = YCbCr422::new(0, 0, 0, 0); // skipped
        image[(0, 8)] = YCbCr422::new(1, 0, 0, 0);
        image[(0, 9)] = YCbCr422::new(1, 0, 0, 0); // skipped
        image[(0, 10)] = YCbCr422::new(2, 0, 0, 0);
        image[(0, 11)] = YCbCr422::new(2, 0, 0, 0); // skipped
        image[(0, 12)] = YCbCr422::new(3, 0, 0, 0);
        image[(0, 13)] = YCbCr422::new(3, 0, 0, 0); // skipped
        image[(0, 14)] = YCbCr422::new(4, 0, 0, 0);
        image[(0, 15)] = YCbCr422::new(4, 0, 0, 0); // skipped

        // segment boundary will be here

        image[(0, 16)] = YCbCr422::new(5, 0, 0, 0);
        image[(0, 17)] = YCbCr422::new(5, 0, 0, 0); // skipped
        image[(0, 18)] = YCbCr422::new(4, 0, 0, 0);
        image[(0, 19)] = YCbCr422::new(4, 0, 0, 0); // skipped
        image[(0, 20)] = YCbCr422::new(3, 0, 0, 0);
        image[(0, 21)] = YCbCr422::new(3, 0, 0, 0); // skipped
        image[(0, 22)] = YCbCr422::new(2, 0, 0, 0);
        image[(0, 23)] = YCbCr422::new(2, 0, 0, 0); // skipped
        image[(0, 24)] = YCbCr422::new(1, 0, 0, 0);
        image[(0, 25)] = YCbCr422::new(1, 0, 0, 0); // skipped

        // segment boundary will be here

        image[(0, 26)] = YCbCr422::new(0, 0, 0, 0);
        image[(0, 27)] = YCbCr422::new(0, 0, 0, 0); // skipped
        image[(0, 28)] = YCbCr422::new(0, 0, 0, 0);
        image[(0, 29)] = YCbCr422::new(0, 0, 0, 0); // skipped
        image[(0, 30)] = YCbCr422::new(0, 0, 0, 0);
        image[(0, 31)] = YCbCr422::new(0, 0, 0, 0); // skipped
        image[(0, 32)] = YCbCr422::new(0, 0, 0, 0);
        image[(0, 33)] = YCbCr422::new(0, 0, 0, 0); // skipped
        image[(0, 34)] = YCbCr422::new(0, 0, 0, 0);
        image[(0, 35)] = YCbCr422::new(0, 0, 0, 0); // skipped
        image[(0, 36)] = YCbCr422::new(0, 0, 0, 0);
        image[(0, 37)] = YCbCr422::new(0, 0, 0, 0); // skipped
        image[(0, 38)] = YCbCr422::new(0, 0, 0, 0);
        image[(0, 39)] = YCbCr422::new(0, 0, 0, 0); // skipped
        image[(0, 40)] = YCbCr422::new(0, 0, 0, 0);
        image[(0, 41)] = YCbCr422::new(0, 0, 0, 0); // skipped
        image[(0, 42)] = YCbCr422::new(0, 0, 0, 0);
        image[(0, 43)] = YCbCr422::new(0, 0, 0, 0); // skipped

        // segment boundary will be here

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

        let scan_line = new_vertical_scan_line(&image, &field_color, 0, 2, 1, false, 0.0);
        assert_eq!(scan_line.position, 0);
        assert_eq!(scan_line.segments.len(), 3);
        assert_eq!(scan_line.segments[0].start, 0);
        assert_eq!(scan_line.segments[0].end, 16);
        assert_eq!(scan_line.segments[0].start_edge_type, EdgeType::Border);
        assert_eq!(scan_line.segments[0].end_edge_type, EdgeType::Rising);
        assert_eq!(scan_line.segments[1].start, 16);
        assert_eq!(scan_line.segments[1].end, 26);
        assert_eq!(scan_line.segments[1].start_edge_type, EdgeType::Rising);
        assert_eq!(scan_line.segments[1].end_edge_type, EdgeType::Falling);
        assert_eq!(scan_line.segments[2].start, 26);
        assert_eq!(scan_line.segments[2].end, 44);
        assert_eq!(scan_line.segments[2].start_edge_type, EdgeType::Falling);
        assert_eq!(scan_line.segments[2].end_edge_type, EdgeType::Border);
    }

    #[test]
    fn image_with_three_vertical_segments_with_higher_differences_with_median() {
        let mut image = Image422::zero(3, 44);
        let field_color = FieldColor {
            red_chromaticity_threshold: 0.37,
            blue_chromaticity_threshold: 0.38,
            lower_green_chromaticity_threshold: 0.4,
            upper_green_chromaticity_threshold: 0.43,
        };
        // only evaluating every secondth 422 pixel
        image[(0, 0)] = YCbCr422::new(0, 0, 0, 0);
        image[(0, 1)] = YCbCr422::new(0, 0, 0, 0); // skipped
        image[(0, 2)] = YCbCr422::new(0, 0, 0, 0);
        image[(0, 3)] = YCbCr422::new(0, 0, 0, 0); // skipped
        image[(0, 4)] = YCbCr422::new(0, 0, 0, 0);
        image[(0, 5)] = YCbCr422::new(0, 0, 0, 0); // skipped
        image[(0, 6)] = YCbCr422::new(0, 0, 0, 0);
        image[(0, 7)] = YCbCr422::new(0, 0, 0, 0); // skipped
        image[(0, 8)] = YCbCr422::new(1, 0, 0, 0);
        image[(0, 9)] = YCbCr422::new(1, 0, 0, 0); // skipped
        image[(0, 10)] = YCbCr422::new(2, 0, 0, 0);
        image[(0, 11)] = YCbCr422::new(2, 0, 0, 0); // skipped
        image[(0, 12)] = YCbCr422::new(3, 0, 0, 0);
        image[(0, 13)] = YCbCr422::new(3, 0, 0, 0); // skipped
        image[(0, 14)] = YCbCr422::new(4, 0, 0, 0);
        image[(0, 15)] = YCbCr422::new(4, 0, 0, 0); // skipped
        image[(0, 16)] = YCbCr422::new(5, 0, 0, 0);

        // segment boundary will be here

        image[(0, 17)] = YCbCr422::new(5, 0, 0, 0); // skipped
        image[(0, 18)] = YCbCr422::new(4, 0, 0, 0);
        image[(0, 19)] = YCbCr422::new(4, 0, 0, 0); // skipped
        image[(0, 20)] = YCbCr422::new(3, 0, 0, 0);
        image[(0, 21)] = YCbCr422::new(3, 0, 0, 0); // skipped
        image[(0, 22)] = YCbCr422::new(2, 0, 0, 0);
        image[(0, 23)] = YCbCr422::new(2, 0, 0, 0); // skipped
        image[(0, 24)] = YCbCr422::new(1, 0, 0, 0);
        image[(0, 25)] = YCbCr422::new(1, 0, 0, 0); // skipped
        image[(0, 26)] = YCbCr422::new(0, 0, 0, 0);

        // segment boundary will be here

        image[(0, 27)] = YCbCr422::new(0, 0, 0, 0); // skipped
        image[(0, 28)] = YCbCr422::new(0, 0, 0, 0);
        image[(0, 29)] = YCbCr422::new(0, 0, 0, 0); // skipped
        image[(0, 30)] = YCbCr422::new(0, 0, 0, 0);
        image[(0, 31)] = YCbCr422::new(0, 0, 0, 0); // skipped
        image[(0, 32)] = YCbCr422::new(0, 0, 0, 0);
        image[(0, 33)] = YCbCr422::new(0, 0, 0, 0); // skipped
        image[(0, 34)] = YCbCr422::new(0, 0, 0, 0);
        image[(0, 35)] = YCbCr422::new(0, 0, 0, 0); // skipped
        image[(0, 36)] = YCbCr422::new(0, 0, 0, 0);
        image[(0, 37)] = YCbCr422::new(0, 0, 0, 0); // skipped
        image[(0, 38)] = YCbCr422::new(0, 0, 0, 0);
        image[(0, 39)] = YCbCr422::new(0, 0, 0, 0); // skipped
        image[(0, 40)] = YCbCr422::new(0, 0, 0, 0);
        image[(0, 41)] = YCbCr422::new(0, 0, 0, 0); // skipped
        image[(0, 42)] = YCbCr422::new(0, 0, 0, 0);
        image[(0, 43)] = YCbCr422::new(0, 0, 0, 0); // skipped

        // segment boundary will be here

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

        let scan_line = new_vertical_scan_line(&image, &field_color, 0, 2, 1, true, 0.0);
        assert_eq!(scan_line.position, 0);
        assert_eq!(scan_line.segments.len(), 3);
        assert_eq!(scan_line.segments[0].start, 0);
        assert_eq!(scan_line.segments[0].end, 17);
        assert_eq!(scan_line.segments[0].start_edge_type, EdgeType::Border);
        assert_eq!(scan_line.segments[0].end_edge_type, EdgeType::Rising);
        assert_eq!(scan_line.segments[1].start, 17);
        assert_eq!(scan_line.segments[1].end, 27);
        assert_eq!(scan_line.segments[1].start_edge_type, EdgeType::Rising);
        assert_eq!(scan_line.segments[1].end_edge_type, EdgeType::Falling);
        assert_eq!(scan_line.segments[2].start, 27);
        assert_eq!(scan_line.segments[2].end, 44);
        assert_eq!(scan_line.segments[2].start_edge_type, EdgeType::Falling);
        assert_eq!(scan_line.segments[2].end_edge_type, EdgeType::Border);
    }

    #[test]
    fn image_with_one_horizontal_segment_with_increasing_differences() {
        let mut image = Image422::zero(16, 3);
        let field_color = FieldColor {
            red_chromaticity_threshold: 0.37,
            blue_chromaticity_threshold: 0.38,
            lower_green_chromaticity_threshold: 0.4,
            upper_green_chromaticity_threshold: 0.43,
        };
        // only evaluating every secondth 422 pixel
        image[(0, 0)] = YCbCr422::new(0, 0, 0, 0);
        image[(1, 0)] = YCbCr422::new(0, 0, 0, 0); // skipped
        image[(2, 0)] = YCbCr422::new(0, 0, 0, 0);
        image[(3, 0)] = YCbCr422::new(0, 0, 0, 0); // skipped
        image[(4, 0)] = YCbCr422::new(1, 0, 0, 0);
        image[(5, 0)] = YCbCr422::new(1, 0, 0, 0); // skipped
        image[(6, 0)] = YCbCr422::new(3, 0, 0, 0);
        image[(7, 0)] = YCbCr422::new(3, 0, 0, 0); // skipped
        image[(8, 0)] = YCbCr422::new(6, 0, 0, 0);
        image[(9, 0)] = YCbCr422::new(6, 0, 0, 0); // skipped
        image[(10, 0)] = YCbCr422::new(10, 0, 0, 0);
        image[(11, 0)] = YCbCr422::new(10, 0, 0, 0); // skipped
        image[(12, 0)] = YCbCr422::new(15, 0, 0, 0);
        image[(13, 0)] = YCbCr422::new(15, 0, 0, 0); // skipped
        image[(14, 0)] = YCbCr422::new(21, 0, 0, 0);
        image[(15, 0)] = YCbCr422::new(21, 0, 0, 0); // skipped

        // segment boundary will be here

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

        let scan_line = new_horizontal_scan_line(&image, &field_color, 0, 2, 1);
        assert_eq!(scan_line.position, 0);
        assert_eq!(scan_line.segments.len(), 1);
        assert_eq!(scan_line.segments[0].start, 0);
        assert_eq!(scan_line.segments[0].end, 16);
        assert_eq!(scan_line.segments[0].start_edge_type, EdgeType::Border);
        assert_eq!(scan_line.segments[0].end_edge_type, EdgeType::Border);
    }

    #[test]
    fn image_with_one_vertical_segment_with_increasing_differences_without_median() {
        let mut image = Image422::zero(3, 16);
        let field_color = FieldColor {
            red_chromaticity_threshold: 0.37,
            blue_chromaticity_threshold: 0.38,
            lower_green_chromaticity_threshold: 0.4,
            upper_green_chromaticity_threshold: 0.43,
        };
        // only evaluating every secondth 422 pixel
        image[(0, 0)] = YCbCr422::new(0, 0, 0, 0);
        image[(0, 1)] = YCbCr422::new(0, 0, 0, 0); // skipped
        image[(0, 2)] = YCbCr422::new(0, 0, 0, 0);
        image[(0, 3)] = YCbCr422::new(0, 0, 0, 0); // skipped
        image[(0, 4)] = YCbCr422::new(1, 0, 0, 0);
        image[(0, 5)] = YCbCr422::new(1, 0, 0, 0); // skipped
        image[(0, 6)] = YCbCr422::new(3, 0, 0, 0);
        image[(0, 7)] = YCbCr422::new(3, 0, 0, 0); // skipped
        image[(0, 8)] = YCbCr422::new(6, 0, 0, 0);
        image[(0, 9)] = YCbCr422::new(6, 0, 0, 0); // skipped
        image[(0, 10)] = YCbCr422::new(10, 0, 0, 0);
        image[(0, 11)] = YCbCr422::new(10, 0, 0, 0); // skipped
        image[(0, 12)] = YCbCr422::new(15, 0, 0, 0);
        image[(0, 13)] = YCbCr422::new(15, 0, 0, 0); // skipped
        image[(0, 14)] = YCbCr422::new(21, 0, 0, 0);
        image[(0, 15)] = YCbCr422::new(21, 0, 0, 0); // skipped

        // segment boundary will be here

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

        let scan_line = new_vertical_scan_line(&image, &field_color, 0, 2, 1, false, 0.0);
        assert_eq!(scan_line.position, 0);
        assert_eq!(scan_line.segments.len(), 1);
        assert_eq!(scan_line.segments[0].start, 0);
        assert_eq!(scan_line.segments[0].end, 16);
        assert_eq!(scan_line.segments[0].start_edge_type, EdgeType::Border);
        assert_eq!(scan_line.segments[0].end_edge_type, EdgeType::Border);
    }

    #[test]
    fn image_with_one_vertical_segment_with_increasing_differences_with_median() {
        let mut image = Image422::zero(3, 16);
        let field_color = FieldColor {
            red_chromaticity_threshold: 0.37,
            blue_chromaticity_threshold: 0.38,
            lower_green_chromaticity_threshold: 0.4,
            upper_green_chromaticity_threshold: 0.43,
        };
        // only evaluating every secondth 422 pixel
        image[(0, 0)] = YCbCr422::new(0, 0, 0, 0);
        image[(0, 1)] = YCbCr422::new(0, 0, 0, 0); // skipped
        image[(0, 2)] = YCbCr422::new(0, 0, 0, 0);
        image[(0, 3)] = YCbCr422::new(0, 0, 0, 0); // skipped
        image[(0, 4)] = YCbCr422::new(1, 0, 0, 0);
        image[(0, 5)] = YCbCr422::new(1, 0, 0, 0); // skipped
        image[(0, 6)] = YCbCr422::new(3, 0, 0, 0);
        image[(0, 7)] = YCbCr422::new(3, 0, 0, 0); // skipped
        image[(0, 8)] = YCbCr422::new(6, 0, 0, 0);
        image[(0, 9)] = YCbCr422::new(6, 0, 0, 0); // skipped
        image[(0, 10)] = YCbCr422::new(10, 0, 0, 0);
        image[(0, 11)] = YCbCr422::new(10, 0, 0, 0); // skipped
        image[(0, 12)] = YCbCr422::new(15, 0, 0, 0);
        image[(0, 13)] = YCbCr422::new(15, 0, 0, 0); // skipped
        image[(0, 14)] = YCbCr422::new(21, 0, 0, 0);
        image[(0, 15)] = YCbCr422::new(21, 0, 0, 0); // skipped

        // segment boundary will be here

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

        let scan_line = new_vertical_scan_line(&image, &field_color, 0, 2, 1, true, 0.0);
        assert_eq!(scan_line.position, 0);
        assert_eq!(scan_line.segments.len(), 1);
        assert_eq!(scan_line.segments[0].start, 0);
        assert_eq!(scan_line.segments[0].end, 16);
        assert_eq!(scan_line.segments[0].start_edge_type, EdgeType::Border);
        assert_eq!(scan_line.segments[0].end_edge_type, EdgeType::Border);
    }

    #[test]
    fn median_of_three_with_same_values() {
        // first == second == third
        assert_eq!(median_of_three(0, 0, 0), 0);
        // first < second == third
        assert_eq!(median_of_three(0, 1, 1), 1);
        // first == second < third
        assert_eq!(median_of_three(0, 0, 1), 0);
        // first == third < second
        assert_eq!(median_of_three(0, 1, 0), 0);
    }

    #[test]
    fn median_of_three_with_different_values() {
        // first <= second <= third
        assert_eq!(median_of_three(0, 1, 2), 1);
        // first <= third < second
        assert_eq!(median_of_three(0, 2, 1), 1);
        // third < first <= second
        assert_eq!(median_of_three(1, 2, 0), 1);
        // second < first <= third
        assert_eq!(median_of_three(1, 0, 2), 1);
        // second <= third < first
        assert_eq!(median_of_three(2, 0, 1), 1);
        // third < second <= first
        assert_eq!(median_of_three(2, 1, 0), 1);
    }
}
