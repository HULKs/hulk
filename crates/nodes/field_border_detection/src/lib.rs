use std::sync::Arc;

use color_eyre::Result;
use geometry::line_segment::LineSegment;
use rand::SeedableRng;
use rand_chacha::ChaChaRng;
use ransac::Ransac;

use coordinate_systems::Pixel;
use linear_algebra::{Point2, Vector2, point};
use projection::{Projection, camera_matrix::CameraMatrix};
use ros_z::prelude::*;
use types::{
    color::Intensity,
    field_border::FieldBorder,
    image_segments::{ImageSegments, ScanLine, Segment},
    parameters::FieldBorderDetectionParameters,
    time_wrapper::TimeWrapper,
};

pub async fn run(ctx: Arc<Context>) -> Result<()> {
    let node = ctx.create_node("field_border_detection").build().await?;

    let parameters =
        node.bind_parameter_as::<FieldBorderDetectionParameters>("field_border_detection")?;
    let camera_matrix_cache = node
        .create_cache::<TimeWrapper<CameraMatrix>>("camera_matrix", 20)?
        .with_stamp(|w: &TimeWrapper<CameraMatrix>| w.time)
        .build()
        .await?;
    let image_segments_sub = node
        .subscriber::<TimeWrapper<ImageSegments>>("image_segments")?
        .build()
        .await?;
    let field_border_points_pub = node
        .publisher::<Vec<Point2<Pixel>>>("field_border_points")?
        .build()
        .await?;
    let field_border_pub = node
        .publisher::<TimeWrapper<Option<FieldBorder>>>("field_border")?
        .build()
        .await?;

    let mut random_state = ChaChaRng::from_os_rng();

    loop {
        let parameters = parameters.snapshot().typed().clone();
        if !parameters.enable {
            continue;
        }

        let timed_image_segments = image_segments_sub.recv().await?;
        let time_stamp = timed_image_segments.time;
        let image_segments = timed_image_segments.inner;

        let Some(timed_camera_matrix) = camera_matrix_cache.get_nearest(time_stamp) else {
            continue;
        };
        let camera_matrix = &timed_camera_matrix.inner;

        let first_field_pixels: Vec<_> = image_segments
            .scan_grid
            .vertical_scan_lines
            .iter()
            .filter_map(|scan_line| {
                get_first_field_segment(scan_line)
                    .map(|segment| point![scan_line.position as f32, segment.start as f32])
            })
            .collect();

        field_border_points_pub.publish(&first_field_pixels).await?;

        let ransac = Ransac::new(first_field_pixels);
        let border_lines =
            find_border_lines(ransac, &mut random_state, camera_matrix, &parameters);

        field_border_pub
            .publish(&TimeWrapper {
                time: time_stamp,
                inner: Some(FieldBorder { border_lines }),
            })
            .await?;
    }
}

fn get_first_field_segment(scan_line: &ScanLine) -> Option<&Segment> {
    scan_line
        .segments
        .iter()
        .find(|segment| segment.field_color == Intensity::High)
}

fn find_border_lines(
    mut ransac: Ransac<Pixel>,
    random_state: &mut ChaChaRng,
    camera_matrix: &CameraMatrix,
    parameters: &FieldBorderDetectionParameters,
) -> Vec<LineSegment<Pixel>> {
    // first line
    let result = ransac.next_line(
        random_state,
        20,
        parameters.first_line_association_distance,
        parameters.first_line_association_distance,
    );
    if result.line.is_none() || result.used_points.len() < parameters.min_points_per_line {
        return Vec::new();
    }
    let first_line = best_fit_line(&result.used_points);
    // second line
    let result = ransac.next_line(
        random_state,
        20,
        parameters.second_line_association_distance,
        parameters.second_line_association_distance,
    );
    if result.line.is_none() || result.used_points.len() < parameters.min_points_per_line {
        return vec![first_line];
    }
    let second_line = best_fit_line(&result.used_points);
    if !is_orthogonal(
        &[first_line, second_line],
        camera_matrix,
        parameters.angle_threshold,
    )
    .unwrap_or(false)
    {
        return vec![first_line];
    }
    vec![first_line, second_line]
}

fn best_fit_line(points: &[Point2<Pixel>]) -> LineSegment<Pixel> {
    let half_size = points.len() / 2;
    let line_start = find_center_of_group(&points[0..half_size]);
    let line_end = find_center_of_group(&points[half_size..points.len()]);
    LineSegment(line_start, line_end)
}

fn find_center_of_group(group: &[Point2<Pixel>]) -> Point2<Pixel> {
    group
        .iter()
        .map(|point| point.coords())
        .sum::<Vector2<_>>()
        .unscale(group.len() as f32)
        .as_point()
}

fn is_orthogonal(
    lines: &[LineSegment<Pixel>; 2],
    camera_matrix: &CameraMatrix,
    angle_threshold: f32,
) -> Result<bool> {
    let projected_lines = [
        LineSegment(
            camera_matrix.pixel_to_ground(lines[0].0)?,
            camera_matrix.pixel_to_ground(lines[0].1)?,
        ),
        LineSegment(
            camera_matrix.pixel_to_ground(lines[1].0)?,
            camera_matrix.pixel_to_ground(lines[1].1)?,
        ),
    ];
    Ok(projected_lines[0].is_orthogonal(projected_lines[1], angle_threshold))
}

#[cfg(test)]
mod test {
    use approx::assert_relative_eq;
    use linear_algebra::point;
    use rand::{Rng, SeedableRng, rngs::StdRng};
    use types::{
        color::{Intensity, YCbCr444},
        image_segments::{EdgeType, ScanLine, Segment},
    };

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
        let green_segment = get_first_field_segment(&scanline);
        assert_eq!(green_segment, Some(&scanline.segments[7]));
    }

    #[test]
    fn find_centre_of_two_points() {
        let points = vec![point![2.0, 5.0], point![4.0, 7.0]];
        let centre = find_center_of_group(&points);
        assert_relative_eq!(centre, point![3.0, 6.0]);
    }

    #[test]
    fn centre_of_mirrored_point_cloud() {
        let mut random_number_generator = StdRng::seed_from_u64(0);
        let centre = point![
            random_number_generator.random_range(-100.0..100.0),
            random_number_generator.random_range(-100.0..100.0)
        ];
        let points: Vec<_> = (0..50)
            .flat_map(|_| {
                let new_point = point![
                    random_number_generator.random_range(-100.0..100.0),
                    random_number_generator.random_range(-100.0..100.0)
                ];
                let new_mirrored_point = centre + (centre - new_point);
                vec![new_point, new_mirrored_point]
            })
            .collect();
        let calculated_centre = find_center_of_group(&points);
        assert_relative_eq!(centre, calculated_centre, epsilon = 0.0001);
    }
}
