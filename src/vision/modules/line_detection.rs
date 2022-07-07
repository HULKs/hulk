use std::{collections::HashSet, ops::Range};

use module_derive::{module, require_some};
use nalgebra::{distance, point, vector, Point2, Vector2};
use ordered_float::NotNan;
use types::{CameraMatrix, EdgeType, FilteredSegments, ImageLines, Line, LineData, Segment};

use crate::{Ransac, RansacResult};

pub struct LineDetection;

#[module(vision)]
#[input(path = camera_matrix, data_type = CameraMatrix)]
#[input(path = filtered_segments, data_type = FilteredSegments)]
#[parameter(path = $this_cycler.line_detection.allowed_line_length_in_field, data_type = Range<f32>)]
#[parameter(path = $this_cycler.line_detection.check_line_segments_projection, data_type = bool)]
#[parameter(path = $this_cycler.line_detection.check_line_length, data_type = bool)]
#[parameter(path = $this_cycler.line_detection.check_line_distance, data_type = bool)]
#[parameter(path = $this_cycler.line_detection.gradient_alignment, data_type = f32)]
#[parameter(path = $this_cycler.line_detection.maximum_distance_to_robot, data_type = f32)]
#[parameter(path = $this_cycler.line_detection.maximum_fit_distance_in_pixels, data_type = f32)]
#[parameter(path = $this_cycler.line_detection.maximum_gap_on_line, data_type = f32)]
#[parameter(path = $this_cycler.line_detection.maximum_number_of_lines, data_type = usize)]
#[parameter(path = $this_cycler.line_detection.maximum_projected_segment_length, data_type = f32)]
#[parameter(path = $this_cycler.line_detection.minimum_number_of_points_on_line, data_type = usize)]
#[additional_output(path = lines_in_image, data_type = ImageLines)]
#[main_output(data_type = LineData)]
impl LineDetection {}

impl LineDetection {
    fn new(_context: NewContext) -> anyhow::Result<Self> {
        Ok(Self)
    }

    fn cycle(&mut self, mut context: CycleContext) -> anyhow::Result<MainOutputs> {
        let camera_matrix = require_some!(context.camera_matrix);
        let filtered_segments = require_some!(context.filtered_segments);

        let mut lines_in_image = ImageLines {
            lines: vec![],
            points: vec![],
        };

        let (line_points, used_vertical_filtered_segments) =
            filter_segments_for_lines(camera_matrix, filtered_segments, &context);
        if context.lines_in_image.is_subscribed() {
            lines_in_image.points = line_points.clone();
        }
        let mut ransac = Ransac::new(line_points);
        let mut lines_in_robot = Vec::new();
        for _ in 0..*context.maximum_number_of_lines {
            if ransac.unused_points.len() < *context.minimum_number_of_points_on_line {
                break;
            }
            let RansacResult {
                line: ransac_line,
                used_points,
            } = ransac.next_line(20, *context.maximum_fit_distance_in_pixels);
            let ransac_line =
                ransac_line.expect("Insufficient number of line points. Cannot fit line.");
            if used_points.len() < *context.minimum_number_of_points_on_line {
                break;
            }
            let mut points_with_projection_onto_line: Vec<_> = used_points
                .iter()
                .map(|&point| (point, ransac_line.project_point(point)))
                .collect();
            points_with_projection_onto_line.sort_by_key(|(_point, projected_point)| {
                NotNan::new(projected_point.x).expect("Tried to compare NaN")
            });
            let split_index = (1..points_with_projection_onto_line.len())
                .find(|&index| {
                    distance(
                        &points_with_projection_onto_line[index - 1].1,
                        &points_with_projection_onto_line[index].1,
                    ) > *context.maximum_gap_on_line
                })
                .unwrap_or(points_with_projection_onto_line.len());
            let after_gap = points_with_projection_onto_line.split_off(split_index);
            ransac
                .unused_points
                .extend(after_gap.iter().map(|(point, _projected_point)| point));
            if points_with_projection_onto_line.len() < *context.minimum_number_of_points_on_line {
                // just drop and ignore this line
                continue;
            }
            let (start_point_in_image, start_point_in_robot) =
                match points_with_projection_onto_line.iter().find_map(
                    |(point, projected_point)| {
                        let projected_point_444 =
                            point![projected_point.x * 2.0, projected_point.y];
                        Some((
                            *point,
                            camera_matrix.pixel_to_ground(&projected_point_444).ok()?,
                        ))
                    },
                ) {
                    Some(start) => start,
                    None => break,
                };
            let (end_point_in_image, end_point_in_robot) = match points_with_projection_onto_line
                .iter()
                .rev()
                .find_map(|(point, projected_point)| {
                    let projected_point_444 = point![projected_point.x * 2.0, projected_point.y];
                    Some((
                        *point,
                        camera_matrix.pixel_to_ground(&projected_point_444).ok()?,
                    ))
                }) {
                Some(end) => end,
                None => break,
            };

            let line_in_robot = Line(start_point_in_robot, end_point_in_robot);
            let line_length_in_robot = line_in_robot.length();
            let is_too_short = *context.check_line_length
                && line_length_in_robot < context.allowed_line_length_in_field.start;
            let is_too_long = *context.check_line_length
                && line_length_in_robot > context.allowed_line_length_in_field.end;
            if is_too_short || is_too_long {
                continue;
            }

            let is_too_far = *context.check_line_distance
                && line_in_robot.center().coords.norm() > *context.maximum_distance_to_robot;
            if is_too_far {
                continue;
            }

            lines_in_robot.push(line_in_robot);
            if context.lines_in_image.is_subscribed() {
                lines_in_image
                    .lines
                    .push(Line(start_point_in_image, end_point_in_image));
            }
        }
        let line_data = LineData {
            lines_in_robot,
            used_vertical_filtered_segments,
        };
        context
            .lines_in_image
            .fill_on_subscription(|| lines_in_image);
        Ok(MainOutputs {
            line_data: Some(line_data),
        })
    }
}

fn get_gradient(image: &Image422, point: Point2<u16>) -> Vector2<f32> {
    if point.x < 1
        || point.y < 1
        || point.x > image.width() as u16 - 2
        || point.y > image.height() as u16 - 2
    {
        return vector![0.0, 0.0];
    }
    let px = point.x as usize;
    let py = point.y as usize;
    // Sobel matrix x (transposed)
    // -1 -2 -1
    //  0  0  0
    //  1  2  1
    let gradient_x = (-1.0 * image[(px - 1, py - 1)].y2 as f32)
        + (-2.0 * image[(px, py - 1)].y1 as f32)
        + (-1.0 * image[(px, py - 1)].y2 as f32)
        + (1.0 * image[(px - 1, py + 1)].y2 as f32)
        + (2.0 * image[(px, py + 1)].y1 as f32)
        + (1.0 * image[(px, py + 1)].y2 as f32);
    // Sobel matrix y (transposed)
    //  1  0 -1
    //  2  0 -2
    //  1  0 -1
    let gradient_y = (1.0 * image[(px - 1, py - 1)].y2 as f32)
        + (-1.0 * image[(px, py - 1)].y2 as f32)
        + (2.0 * image[(px - 1, py)].y2 as f32)
        + (-2.0 * image[(px, py)].y2 as f32)
        + (1.0 * image[(px - 1, py + 1)].y2 as f32)
        + (-1.0 * image[(px, py + 1)].y2 as f32);
    let gradient = vector![gradient_x, gradient_y];
    gradient
        .try_normalize(0.0001)
        .unwrap_or_else(Vector2::zeros)
}

fn filter_segments_for_lines(
    camera_matrix: &CameraMatrix,
    filtered_segments: &FilteredSegments,
    context: &CycleContext,
) -> (Vec<Point2<f32>>, HashSet<Point2<u16>>) {
    let (line_points, used_vertical_filtered_segments) = filtered_segments
        .scan_grid
        .vertical_scan_lines
        .iter()
        .flat_map(|scan_line| {
            let scan_line_position = scan_line.position;
            scan_line.segments.iter().filter_map(move |segment| {
                let is_line_segment = is_line_segment(
                    segment,
                    scan_line_position,
                    context.image,
                    camera_matrix,
                    *context.check_line_segments_projection,
                    *context.maximum_projected_segment_length,
                    *context.gradient_alignment,
                );
                if is_line_segment {
                    Some((scan_line_position, segment))
                } else {
                    None
                }
            })
        })
        .map(|(scan_line_position, segment)| {
            let center = (segment.start + segment.end) as f32 / 2.0;
            (
                point![scan_line_position as f32, center],
                point![scan_line_position, segment.start],
            )
        })
        .unzip();
    (line_points, used_vertical_filtered_segments)
}

fn is_line_segment(
    segment: &Segment,
    scan_line_position: u16,
    image: &Image422,
    camera_matrix: &CameraMatrix,
    check_line_segments_projection: bool,
    maximum_projected_segment_length: f32,
    gradient_alignment: f32,
) -> bool {
    if segment.start_edge_type != EdgeType::Rising || segment.end_edge_type != EdgeType::Falling {
        return false;
    }
    let is_too_long = check_line_segments_projection
        && !is_segment_shorter_than(
            camera_matrix,
            point![scan_line_position as f32 * 2.0, segment.start as f32],
            point![scan_line_position as f32 * 2.0, segment.end as f32],
            maximum_projected_segment_length,
        )
        .unwrap_or(false);
    if is_too_long {
        return false;
    }
    // gradients (approximately) point in opposite directions if their dot product is (close to) -1
    let gradient_at_start = get_gradient(image, point![scan_line_position, segment.start]);
    let gradient_at_end = get_gradient(image, point![scan_line_position, segment.end]);
    gradient_at_start.dot(&gradient_at_end) < gradient_alignment
}

fn is_segment_shorter_than(
    camera_matrix: &CameraMatrix,
    segment_start: Point2<f32>,
    segment_end: Point2<f32>,
    maximum_projected_segment_length: f32,
) -> Option<bool> {
    let start_robot_coordinates = camera_matrix.pixel_to_ground(&segment_start).ok()?;
    let end_robot_coordinates = camera_matrix.pixel_to_ground(&segment_end).ok()?;
    Some(
        distance(&start_robot_coordinates, &end_robot_coordinates)
            <= maximum_projected_segment_length,
    )
}

#[cfg(test)]
mod tests {
    use nalgebra::{vector, Isometry3, Translation, UnitQuaternion};
    use types::{CameraPosition, Intensity, ScanGrid, ScanLine, Segment, YCbCr422, YCbCr444};

    use crate::framework::AdditionalOutput;

    use super::*;

    #[test]
    fn check_correct_number_of_line_points() {
        fn create_scanline(
            color: YCbCr444,
            number_of_segments: u16,
            segment_size: u16,
            position: u16,
        ) -> ScanLine {
            let mut segments = Vec::<Segment>::new();
            for i in 0..number_of_segments {
                let mut segment = Segment {
                    start: i * segment_size,
                    end: (i + 1) * segment_size,
                    start_edge_type: EdgeType::Rising,
                    end_edge_type: EdgeType::Falling,
                    color,
                    field_color: Intensity::Low,
                };
                if i == 0 {
                    segment.start_edge_type = EdgeType::ImageBorder;
                }
                if i == number_of_segments - 1 {
                    segment.end_edge_type = EdgeType::ImageBorder;
                }
                if i % 2 == 0 {
                    segment.start_edge_type = EdgeType::Falling;
                    segment.end_edge_type = EdgeType::Rising;
                }
                segments.push(segment);
            }
            ScanLine { position, segments }
        }

        fn create_filtered_segments(
            number_of_scanlines: u16,
            color: YCbCr444,
            number_of_segments: u16,
            segment_size: u16,
        ) -> FilteredSegments {
            let vertical_scan_lines = (0..number_of_scanlines)
                .map(|position| create_scanline(color, number_of_segments, segment_size, position))
                .collect();
            FilteredSegments {
                scan_grid: ScanGrid {
                    vertical_scan_lines,
                },
            }
        }

        fn create_image(width: usize, height: usize) -> Image422 {
            let mut image = Image422::zero(width, height);
            for x in 0..image.width() {
                for y in 0..image.height() {
                    if (y / 10) % 2 == 0 {
                        image[(x, y)] = YCbCr422::new(255, 0, 0, 0);
                    }
                }
            }
            image
        }

        let image_size = vector![10.0, 500.0];

        let image = create_image(image_size.x as usize, image_size.y as usize);
        // let mut camera_matrix = CameraMatrix::matrix_from_parameters(
        let camera_matrix = CameraMatrix::from_normalized_focal_and_center(
            vector![2.0, 2.0],
            point![1.0, 1.0],
            image_size,
            Isometry3 {
                rotation: UnitQuaternion::from_euler_angles(0.0, std::f32::consts::PI / 4.0, 0.0),
                translation: Translation::from(point![0.0, 0.0, 0.5]),
            },
            Isometry3::identity(),
            Isometry3::identity(),
        );
        let filtered_segments =
            create_filtered_segments(10, YCbCr444 { y: 0, cb: 0, cr: 0 }, 10, 10);
        let mut dummy_data = None;
        let lines_in_image = AdditionalOutput::new(false, &mut dummy_data);
        let context = CycleContext {
            image: &image,
            camera_position: CameraPosition::Top,
            lines_in_image,
            camera_matrix: &Some(camera_matrix),
            filtered_segments: &Some(filtered_segments),
            allowed_line_length_in_field: &(0.3..3.0),
            check_line_distance: &false,
            check_line_length: &false,
            check_line_segments_projection: &false,
            gradient_alignment: &-0.95,
            maximum_distance_to_robot: &0.3,
            maximum_fit_distance_in_pixels: &3.0,
            maximum_gap_on_line: &30.0,
            maximum_number_of_lines: &10,
            maximum_projected_segment_length: &0.3,
            minimum_number_of_points_on_line: &5,
        };

        let (line_points, _) = filter_segments_for_lines(
            context.camera_matrix.as_ref().unwrap(),
            context.filtered_segments.as_ref().unwrap(),
            &context,
        );
        assert_eq!(line_points.len(), 32);
    }

    #[test]
    fn check_fixed_segment_size() {
        let image_size = vector![1.0, 1.0];
        let camera_matrix = CameraMatrix::from_normalized_focal_and_center(
            vector![2.0, 2.0],
            point![1.0, 1.0],
            image_size,
            Isometry3 {
                rotation: UnitQuaternion::from_euler_angles(0.0, std::f32::consts::PI / 4.0, 0.0),
                translation: Translation::from(point![0.0, 0.0, 0.5]),
            },
            Isometry3::identity(),
            Isometry3::identity(),
        );
        let start = point![40.0, 2.0];
        let end = point![40.0, 202.0];
        assert!(!is_segment_shorter_than(&camera_matrix, start, end, 0.3).unwrap_or(false));
        let start2 = point![40.0, 364.0];
        let end2 = point![40.0, 366.0];
        assert!(is_segment_shorter_than(&camera_matrix, start2, end2, 0.3).unwrap_or(false));
    }

    #[test]
    fn gradient_of_zero_image() {
        let image = Image422::zero(3, 3);
        let point = point![1, 1];
        assert_eq!(get_gradient(&image, point), vector![0.0, 0.0]);
    }
}
