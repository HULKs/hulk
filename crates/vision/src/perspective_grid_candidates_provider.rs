use std::collections::HashSet;

use color_eyre::Result;
use serde::{Deserialize, Serialize};

use context_attribute::context;
use coordinate_systems::Pixel;
use framework::{AdditionalOutput, MainOutput};
use geometry::circle::Circle;
use linear_algebra::{point, vector, Point2, Vector2};
use projection::Projection;
use types::{
    camera_matrix::CameraMatrix,
    filtered_segments::FilteredSegments,
    image_segments::{ScanLine, Segment},
    line_data::LineData,
    perspective_grid_candidates::{PerspectiveGridCandidates, Row},
    ycbcr422_image::YCbCr422Image,
};

#[derive(Deserialize, Serialize)]
pub struct PerspectiveGridCandidatesProvider {}

#[context]
pub struct CreationContext {}

#[context]
pub struct CycleContext {
    camera_matrix: RequiredInput<Option<CameraMatrix>, "camera_matrix?">,
    filtered_segments: Input<FilteredSegments, "filtered_segments">,
    line_data: RequiredInput<Option<LineData>, "line_data?">,
    image: Input<YCbCr422Image, "image">,

    ball_radius: Parameter<f32, "field_dimensions.ball_radius">,
    minimum_radius:
        Parameter<f32, "perspective_grid_candidates_provider.$cycler_instance.minimum_radius">,

    rows: AdditionalOutput<Vec<Row>, "rows">,
    horizon_center_y: AdditionalOutput<f32, "horizon_center_y">,
}

#[context]
#[derive(Default)]
pub struct MainOutputs {
    pub perspective_grid_candidates: MainOutput<Option<PerspectiveGridCandidates>>,
}

impl PerspectiveGridCandidatesProvider {
    pub fn new(_context: CreationContext) -> Result<Self> {
        Ok(Self {})
    }

    pub fn cycle(&mut self, mut context: CycleContext) -> Result<MainOutputs> {
        let vertical_scanlines = &context.filtered_segments.scan_grid.vertical_scan_lines;
        let skip_segments = &context.line_data.used_segments;
        let image_size = vector![context.image.width(), context.image.height()];

        context.horizon_center_y.fill_if_subscribed(|| {
            context
                .camera_matrix
                .horizon
                .y_at_x(image_size.x() as f32 / 2.0)
        });

        let rows = generate_rows(
            context.camera_matrix,
            image_size,
            *context.minimum_radius,
            *context.ball_radius,
        );

        let candidates = generate_candidates(vertical_scanlines, skip_segments, &rows);
        context.rows.fill_if_subscribed(|| rows);

        Ok(MainOutputs {
            perspective_grid_candidates: Some(candidates).into(),
        })
    }
}

fn generate_rows(
    camera_matrix: &CameraMatrix,
    image_size: Vector2<Pixel, u32>,
    minimum_radius: f32,
    ball_radius: f32,
) -> Vec<Row> {
    let half_width = image_size.x() as f32 / 2.0;

    let mut row_vertical_center = image_size.y() as f32 - 1.0;

    let mut rows = vec![];

    loop {
        let pixel = point![half_width, row_vertical_center];
        let Ok(radius) = camera_matrix.get_pixel_radius(ball_radius, pixel) else {
            break;
        };

        if radius < minimum_radius {
            break;
        }

        rows.push(Row {
            circle_radius: radius,
            center_y: row_vertical_center,
        });
        row_vertical_center -= 2.0 * radius;
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
    skip_segments: &HashSet<Point2<Pixel, u16>>,
    rows: &[Row],
) -> PerspectiveGridCandidates {
    let mut already_added = HashSet::new();
    let mut candidates = Vec::new();

    for scan_line in vertical_scanlines {
        for segment in &scan_line.segments {
            if skip_segments.contains(&point![scan_line.position, segment.start]) {
                continue;
            }

            let (row_index, row) = match find_matching_row(rows, segment) {
                Some(result) => result,
                None => continue,
            };
            let x = scan_line.position as f32;
            let index_in_row = (x / (row.circle_radius * 2.0)).floor() as usize;
            if already_added.insert((row_index, index_in_row)) {
                candidates.push(Circle {
                    center: point![
                        row.circle_radius + row.circle_radius * 2.0 * index_in_row as f32,
                        row.center_y
                    ],
                    radius: row.circle_radius,
                })
            }
        }
    }

    candidates.sort_by(|a, b| {
        b.center
            .y()
            .partial_cmp(&a.center.y())
            .unwrap()
            .then(a.center.x().partial_cmp(&b.center.x()).unwrap())
    });

    PerspectiveGridCandidates { candidates }
}

#[cfg(test)]
mod tests {
    use std::iter::FromIterator;

    use approx::assert_relative_eq;
    use linear_algebra::{vector, IntoTransform, Isometry3};
    use nalgebra::{Translation, UnitQuaternion};
    use types::{
        camera_matrix::CameraMatrix,
        color::{Intensity, YCbCr444},
        image_segments::EdgeType,
    };

    use super::*;

    #[test]
    fn rows_non_empty() {
        let focal_length = nalgebra::vector![0.95, 1.27];
        let optical_center = nalgebra::point![0.5, 0.5];

        let camera_matrix = CameraMatrix::from_normalized_focal_and_center(
            focal_length,
            optical_center,
            vector![640, 480],
            nalgebra::Isometry3 {
                rotation: UnitQuaternion::from_euler_angles(0.0, 39.7_f32.to_radians(), 0.0),
                translation: Translation::from(nalgebra::point![0.0, 0.0, 0.75]),
            }
            .framed_transform(),
            Isometry3::identity(),
            Isometry3::identity(),
        );
        let minimum_radius = 5.0;

        assert!(!generate_rows(&camera_matrix, vector![512, 512], minimum_radius, 42.0).is_empty());
    }

    #[test]
    fn rows_spaced_correctly() {
        let image_size = vector![512, 512];
        let camera_matrix = CameraMatrix::from_normalized_focal_and_center(
            nalgebra::vector![1.0, 1.0],
            nalgebra::point![0.5, 0.5],
            image_size,
            nalgebra::Isometry3 {
                rotation: UnitQuaternion::from_euler_angles(0.0, std::f32::consts::PI / 4.0, 0.0),
                translation: Translation::from(nalgebra::point![0.0, 0.0, 0.5]),
            }
            .framed_transform(),
            Isometry3::identity(),
            Isometry3::identity(),
        );
        let minimum_radius = 5.0;

        let circles = generate_rows(&camera_matrix, image_size, minimum_radius, 42.0);

        circles.iter().reduce(|previous, current| {
            println!("Previous: {previous:#?}");
            println!("Current: {current:#?}");
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
            position: 42,
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
        let candidates = generate_candidates(&vertical_scan_lines, &skip_segments, &rows);
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
                position: 42,
                segments: segments.clone(),
            },
            ScanLine {
                position: 110,
                segments,
            },
        ];
        let skip_segments = HashSet::from_iter(
            [
                point![0, 18],
                point![42, 5],
                point![42, 45],
                point![110, 5],
                point![110, 18],
            ]
            .map(|point| point),
        );
        let candidates = generate_candidates(&vertical_scan_lines, &skip_segments, &rows);
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
