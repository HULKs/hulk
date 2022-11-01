use std::collections::HashSet;

use module_derive::{module, require_some};
use nalgebra::{point, vector, Point2, Vector2};

use types::{
    CameraMatrix, Circle, FilteredSegments, LineData, PerspectiveGridCandidates, ScanLine, Segment,
};

#[derive(Clone, Copy, Debug)]
struct Row {
    circle_radius: f32,
    center_y: f32,
}

pub struct PerspectiveGridCandidatesProvider;

#[module(vision)]
#[input(path = filtered_segments, data_type = FilteredSegments)]
#[input(path = camera_matrix, data_type = CameraMatrix)]
#[input(path = line_data, data_type = LineData)]
#[parameter(path = $cycler_instance.perspective_grid_candidates_provider.minimum_radius, data_type = f32, name = minimum_radius)]
#[parameter(path = $cycler_instance.perspective_grid_candidates_provider.fallback_radius, data_type = f32, name = fallback_radius)]
#[parameter(path = field_dimensions.ball_radius, data_type = f32, name = ball_radius)]
#[main_output(data_type = PerspectiveGridCandidates)]
impl PerspectiveGridCandidatesProvider {}

impl PerspectiveGridCandidatesProvider {
    fn new(_context: NewContext) -> anyhow::Result<Self> {
        Ok(Self)
    }

    fn cycle(&mut self, context: CycleContext) -> anyhow::Result<MainOutputs> {
        let camera_matrix = require_some!(context.camera_matrix);
        let vertical_scanlines = &require_some!(context.filtered_segments)
            .scan_grid
            .vertical_scan_lines;
        let skip_segments = &require_some!(context.line_data).used_vertical_filtered_segments;
        let image_size = vector![context.image.width(), context.image.height()];

        let rows = Self::generate_rows(
            camera_matrix,
            image_size,
            *context.minimum_radius,
            *context.fallback_radius,
            *context.ball_radius,
        );
        let candidates = Self::generate_candidates(vertical_scanlines, skip_segments, &rows);

        Ok(MainOutputs {
            perspective_grid_candidates: Some(candidates),
        })
    }

    fn generate_rows(
        camera_matrix: &CameraMatrix,
        image_size: Vector2<usize>,
        minimum_radius: f32,
        fallback_radius: f32,
        ball_radius: f32,
    ) -> Vec<Row> {
        let higher_horizon_point =
            if camera_matrix.horizon.left_horizon_y < camera_matrix.horizon.right_horizon_y {
                point![0.0, camera_matrix.horizon.left_horizon_y]
            } else {
                point![
                    image_size.x as f32 - 1.0,
                    camera_matrix.horizon.left_horizon_y
                ]
            };

        let mut radius444 = fallback_radius;
        let mut row_vertical_center = image_size.y as f32 - 1.0;

        let mut rows = vec![];

        while row_vertical_center >= higher_horizon_point.y as f32
            && row_vertical_center + ball_radius > 0.0
        {
            radius444 = camera_matrix
                .get_pixel_radius(
                    ball_radius,
                    &point![higher_horizon_point.x, row_vertical_center],
                    &image_size,
                )
                .unwrap_or(radius444);
            if radius444 < minimum_radius {
                break;
            }
            rows.push(Row {
                circle_radius: radius444,
                center_y: row_vertical_center,
            });
            row_vertical_center -= radius444 * 2.0;
        }
        rows
    }

    fn find_matching_row(rows: &[Row], segment: &Segment) -> Option<(usize, Row)> {
        let center_y = (segment.start as f32 + segment.end as f32) / 2.0;
        rows.iter().enumerate().find_map(|(index, row)| {
            if (row.center_y - center_y).abs() <= row.circle_radius {
                Some((index, *row))
            } else {
                None
            }
        })
    }

    fn generate_candidates(
        vertical_scanlines: &[ScanLine],
        skip_segments: &HashSet<Point2<u16>>,
        rows: &[Row],
    ) -> PerspectiveGridCandidates {
        let mut already_added = HashSet::new();
        let mut candidates = Vec::new();

        for scan_line in vertical_scanlines {
            for segment in &scan_line.segments {
                if skip_segments.contains(&point![scan_line.position, segment.start]) {
                    continue;
                }

                let (row_index, row) = match Self::find_matching_row(rows, segment) {
                    Some(result) => result,
                    None => continue,
                };
                let x_422 = scan_line.position as f32;
                let x_444 = x_422 * 2.0;
                let index_in_row = (x_444 / (row.circle_radius * 2.0)).floor() as usize;
                if already_added.insert((row_index, index_in_row)) {
                    candidates.push(Circle {
                        center: point!(
                            row.circle_radius + row.circle_radius * 2.0 * index_in_row as f32,
                            row.center_y
                        ),
                        radius: row.circle_radius,
                    })
                }
            }
        }

        candidates.sort_by(|a, b| {
            b.center
                .y
                .partial_cmp(&a.center.y)
                .unwrap()
                .then(a.center.x.partial_cmp(&b.center.x).unwrap())
        });

        PerspectiveGridCandidates { candidates }
    }
}

#[cfg(test)]
mod tests {
    use std::iter::FromIterator;

    use approx::assert_relative_eq;
    use nalgebra::{vector, Isometry3, Translation, UnitQuaternion};
    use types::{CameraMatrix, EdgeType, Intensity, ScanLine, Segment, YCbCr444};

    use super::*;

    #[test]
    fn rows_non_empty() {
        let camera_matrix = CameraMatrix::default();
        let minimum_radius = 5.0;

        assert!(!PerspectiveGridCandidatesProvider::generate_rows(
            &camera_matrix,
            vector![512, 512],
            minimum_radius,
            42.0,
            0.05,
        )
        .is_empty());
    }

    #[test]
    fn rows_spaced_correctly() {
        let image_size = vector![512, 512];
        let camera_matrix = CameraMatrix::from_normalized_focal_and_center(
            vector![1.0, 1.0],
            point![0.5, 0.5],
            image_size.map(|element| element as f32),
            Isometry3 {
                rotation: UnitQuaternion::from_euler_angles(0.0, std::f32::consts::PI / 4.0, 0.0),
                translation: Translation::from(point![0.0, 0.0, 0.5]),
            },
            Isometry3::identity(),
            Isometry3::identity(),
        );
        let minimum_radius = 5.0;

        let circles = PerspectiveGridCandidatesProvider::generate_rows(
            &camera_matrix,
            image_size,
            minimum_radius,
            42.0,
            0.05,
        );

        circles.iter().reduce(|previous, current| {
            println!("Previous: {:#?}", previous);
            println!("Current: {:#?}", current);
            assert_relative_eq!(
                f32::abs(current.center_y - previous.center_y),
                previous.circle_radius * 2.0,
                epsilon = 0.001
            );

            current
        });
    }

    #[test]
    fn candidates_correct_single_segment() {
        let rows = vec![
            Row {
                circle_radius: 10.0,
                center_y: 10.0,
            },
            Row {
                circle_radius: 10.0,
                center_y: 30.0,
            },
            Row {
                circle_radius: 10.0,
                center_y: 50.0,
            },
        ];
        let vertical_scan_lines = vec![ScanLine {
            position: 21,
            segments: vec![Segment {
                start: 20,
                end: 50,
                start_edge_type: EdgeType::ImageBorder,
                end_edge_type: EdgeType::ImageBorder,
                color: YCbCr444 { y: 0, cb: 0, cr: 0 },
                field_color: Intensity::Low,
            }],
        }];
        let skip_segments = HashSet::new();
        let candidates = PerspectiveGridCandidatesProvider::generate_candidates(
            &vertical_scan_lines,
            &skip_segments,
            &rows,
        );
        assert_relative_eq!(
            candidates,
            PerspectiveGridCandidates {
                candidates: vec![Circle {
                    center: point![50.0, 30.0],
                    radius: 10.0
                }]
            }
        );
    }

    #[test]
    fn candidates_correct_multi_segment() {
        let rows = vec![
            Row {
                circle_radius: 10.0,
                center_y: 10.0,
            },
            Row {
                circle_radius: 10.0,
                center_y: 30.0,
            },
            Row {
                circle_radius: 10.0,
                center_y: 50.0,
            },
        ];
        let segments = vec![
            Segment {
                start: 5,
                end: 12,
                start_edge_type: EdgeType::ImageBorder,
                end_edge_type: EdgeType::ImageBorder,
                color: YCbCr444 { y: 0, cb: 0, cr: 0 },
                field_color: Intensity::Low,
            },
            Segment {
                start: 18,
                end: 28,
                start_edge_type: EdgeType::ImageBorder,
                end_edge_type: EdgeType::ImageBorder,
                color: YCbCr444 { y: 0, cb: 0, cr: 0 },
                field_color: Intensity::Low,
            },
            Segment {
                start: 45,
                end: 50,
                start_edge_type: EdgeType::ImageBorder,
                end_edge_type: EdgeType::ImageBorder,
                color: YCbCr444 { y: 0, cb: 0, cr: 0 },
                field_color: Intensity::Low,
            },
        ];
        let vertical_scan_lines = vec![
            ScanLine {
                position: 0,
                segments: segments.clone(),
            },
            ScanLine {
                position: 21,
                segments: segments.clone(),
            },
            ScanLine {
                position: 55,
                segments,
            },
        ];
        let skip_segments = HashSet::from_iter(
            vec![
                point![0, 18],
                point![21, 5],
                point![21, 45],
                point![55, 5],
                point![55, 18],
            ]
            .into_iter(),
        );
        let candidates = PerspectiveGridCandidatesProvider::generate_candidates(
            &vertical_scan_lines,
            &skip_segments,
            &rows,
        );
        assert_relative_eq!(
            candidates,
            PerspectiveGridCandidates {
                candidates: vec![
                    Circle {
                        center: point![10.0, 50.0],
                        radius: 10.0
                    },
                    Circle {
                        center: point![110.0, 50.0],
                        radius: 10.0
                    },
                    Circle {
                        center: point![50.0, 30.0],
                        radius: 10.0
                    },
                    Circle {
                        center: point![10.0, 10.0],
                        radius: 10.0
                    },
                ]
            }
        );
    }
}
