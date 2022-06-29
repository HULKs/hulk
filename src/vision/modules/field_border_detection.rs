use anyhow::Result;

use macros::{module, require_some};
use nalgebra::{point, Point2, Vector2};

use crate::{
    types::{CameraMatrix, FieldBorder, ImageSegments, Intensity, Line, Line2, Segment},
    Ransac,
};

pub struct FieldBorderDetection;

#[module(vision)]
#[input(path = image_segments, data_type = ImageSegments)]
#[input(path = camera_matrix, data_type = CameraMatrix)]
#[parameter(path = $this_cycler.field_border_detection.min_points_per_line, data_type = usize)]
#[parameter(path = $this_cycler.field_border_detection.angle_threshold, data_type = f32)]
#[parameter(path = $this_cycler.field_border_detection.first_line_association_distance, data_type = f32)]
#[parameter(path = $this_cycler.field_border_detection.second_line_association_distance, data_type = f32)]
#[additional_output(path = field_border_points, data_type = Vec<Point2<f32>>)]
#[main_output(data_type = FieldBorder)]
impl FieldBorderDetection {}

impl FieldBorderDetection {
    fn new(_context: NewContext) -> anyhow::Result<Self> {
        Ok(Self)
    }

    fn cycle(&mut self, mut context: CycleContext) -> anyhow::Result<MainOutputs> {
        let image_segments = require_some!(context.image_segments);
        let camera_matrix = require_some!(context.camera_matrix);

        let first_field_pixels: Vec<_> = image_segments
            .scan_grid
            .vertical_scan_lines
            .iter()
            .filter_map(|scan_line| {
                get_first_field_segment(&scan_line.segments)
                    .map(|segment| point![scan_line.position as f32, segment.start as f32])
            })
            .collect();
        context
            .field_border_points
            .fill_on_subscription(|| first_field_pixels.clone());
        let ransac = Ransac::new(first_field_pixels);
        let border_lines = find_border_lines(
            ransac,
            camera_matrix,
            *context.min_points_per_line,
            *context.angle_threshold,
            *context.first_line_association_distance,
            *context.second_line_association_distance,
        );
        Ok(MainOutputs {
            field_border: Some(FieldBorder { border_lines }),
        })
    }
}

fn get_first_field_segment(segments: &[Segment]) -> Option<&Segment> {
    segments
        .iter()
        .find(|segment| segment.field_color != Intensity::Low)
}

fn find_border_lines(
    mut ransac: Ransac,
    camera_matrix: &CameraMatrix,
    min_points_per_line: usize,
    angle_threshold: f32,
    first_line_association_distance: f32,
    second_line_association_distance: f32,
) -> Vec<Line2> {
    // first line
    let result = ransac.next_line(20, first_line_association_distance);
    if result.line.is_none() || result.used_points.len() < min_points_per_line {
        return Vec::new();
    }
    let first_line = best_fit_line(&result.used_points);
    // second line
    let result = ransac.next_line(20, second_line_association_distance);
    if result.line.is_none() || result.used_points.len() < min_points_per_line {
        return vec![first_line];
    }
    let second_line = best_fit_line(&result.used_points);
    if !is_orthogonal(&[first_line, second_line], camera_matrix, angle_threshold).unwrap_or(false) {
        return vec![first_line];
    }
    vec![first_line, second_line]
}

fn best_fit_line(points: &[Point2<f32>]) -> Line2 {
    let half_size = points.len() / 2;
    let line_start = find_centre_of_group(&points[0..half_size]);
    let line_end = find_centre_of_group(&points[half_size..points.len()]);
    Line(line_start, line_end)
}

fn find_centre_of_group(group: &[Point2<f32>]) -> Point2<f32> {
    Point2::<f32> {
        coords: group
            .iter()
            .map(|point| point.coords)
            .sum::<Vector2<f32>>()
            .unscale(group.len() as f32),
    }
}

fn is_orthogonal(
    lines: &[Line2; 2],
    camera_matrix: &CameraMatrix,
    angle_threshold: f32,
) -> Result<bool> {
    let projected_lines = [
        Line(
            camera_matrix.pixel_to_ground(&lines[0].0)?,
            camera_matrix.pixel_to_ground(&lines[0].1)?,
        ),
        Line(
            camera_matrix.pixel_to_ground(&lines[1].0)?,
            camera_matrix.pixel_to_ground(&lines[1].1)?,
        ),
    ];
    Ok(projected_lines[0].is_orthogonal(&projected_lines[1], angle_threshold))
}

#[cfg(test)]
mod test {
    use approx::assert_relative_eq;
    use rand::{rngs::StdRng, Rng, SeedableRng};

    use crate::types::{EdgeType, ScanLine, YCbCr444};

    use super::*;

    fn create_scanline(color: YCbCr444, number_of_segments: u16, segment_size: u16) -> ScanLine {
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
                segment.start_edge_type = EdgeType::Border;
            }
            if i == number_of_segments - 1 {
                segment.end_edge_type = EdgeType::Border;
            }
            if i % 2 == 0 {
                segment.start_edge_type = EdgeType::Falling;
                segment.end_edge_type = EdgeType::Rising;
            }
            segments.push(segment);
        }
        ScanLine {
            position: 0,
            segments,
        }
    }

    #[test]
    fn find_first_field_segment_in_single_scanline() {
        let mut scanline = create_scanline(
            YCbCr444 {
                y: 20,
                cb: 100,
                cr: 150,
            },
            10,
            25,
        );
        scanline.segments[7].field_color = Intensity::High;
        let green_segment = get_first_field_segment(&scanline.segments);
        assert_eq!(green_segment, Some(&scanline.segments[7]));
    }

    #[test]
    fn find_centre_of_two_points() {
        let points = vec![Point2::<f32>::new(2.0, 5.0), Point2::<f32>::new(4.0, 7.0)];
        let centre = find_centre_of_group(&points);
        assert_relative_eq!(centre, Point2::<f32>::new(3.0, 6.0));
    }

    #[test]
    fn centre_of_mirrored_point_cloud() {
        let mut random_number_generator = StdRng::seed_from_u64(0);
        let centre = point![
            random_number_generator.gen_range(-100.0..100.0),
            random_number_generator.gen_range(-100.0..100.0)
        ];
        let points: Vec<Point2<f32>> = (0..50)
            .flat_map(|_| {
                let new_point = point![
                    random_number_generator.gen_range(-100.0..100.0),
                    random_number_generator.gen_range(-100.0..100.0)
                ];
                let new_mirrored_point = centre + (centre - new_point);
                vec![new_point, new_mirrored_point]
            })
            .collect();
        let calculated_centre = find_centre_of_group(&points);
        assert_relative_eq!(centre, calculated_centre, epsilon = 0.0001);
    }
}
