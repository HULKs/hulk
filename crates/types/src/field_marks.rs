use nalgebra::{distance, point, vector, Point2, Vector2};
use ordered_float::NotNan;
use serde::{Deserialize, Serialize};

use crate::{
    field_dimensions::FieldDimensions,
    line::{Line, Line2},
};

#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
pub enum FieldMark {
    Line { line: Line2, direction: Direction },
    Circle { center: Point2<f32>, radius: f32 },
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
pub enum Direction {
    PositiveX,
    PositiveY,
}

impl FieldMark {
    pub fn to_correspondence_points(self, measured_line: Line2) -> Correspondences {
        match self {
            FieldMark::Line {
                line: reference_line,
                direction: _,
            } => {
                let measured_line = match [
                    distance(&measured_line.0, &reference_line.0),
                    distance(&measured_line.0, &reference_line.1),
                    distance(&measured_line.1, &reference_line.0),
                    distance(&measured_line.1, &reference_line.1),
                ]
                .iter()
                .enumerate()
                .min_by_key(|(_index, distance)| NotNan::new(**distance).unwrap())
                .unwrap()
                .0
                {
                    1 | 2 => Line(measured_line.1, measured_line.0),
                    _ => measured_line,
                };

                let measured_direction = (measured_line.0 - measured_line.1).normalize();
                let reference_direction = (reference_line.0 - reference_line.1).normalize();

                let projected_point_on_measured_line =
                    measured_line.project_onto_segment(reference_line.0);
                let projected_point_on_reference_line =
                    reference_line.project_onto_segment(measured_line.0);

                let measured_distance =
                    distance(&projected_point_on_measured_line, &reference_line.0);
                let reference_distance =
                    distance(&measured_line.0, &projected_point_on_reference_line);
                let correspondence_0 = if measured_distance < reference_distance {
                    CorrespondencePoints {
                        measured: projected_point_on_measured_line,
                        reference: reference_line.0,
                    }
                } else {
                    CorrespondencePoints {
                        measured: measured_line.0,
                        reference: projected_point_on_reference_line,
                    }
                };

                let projected_point_on_measured_line =
                    measured_line.project_onto_segment(reference_line.1);
                let projected_point_on_reference_line =
                    reference_line.project_onto_segment(measured_line.1);

                let measured_distance =
                    distance(&projected_point_on_measured_line, &reference_line.1);
                let reference_distance =
                    distance(&measured_line.1, &projected_point_on_reference_line);
                let correspondence_1 = if measured_distance < reference_distance {
                    CorrespondencePoints {
                        measured: projected_point_on_measured_line,
                        reference: reference_line.1,
                    }
                } else {
                    CorrespondencePoints {
                        measured: measured_line.1,
                        reference: projected_point_on_reference_line,
                    }
                };

                Correspondences {
                    correspondence_points: (correspondence_0, correspondence_1),
                    measured_direction,
                    reference_direction,
                }
            }
            FieldMark::Circle { center, radius } => {
                let center_to_0 = measured_line.0 - center;
                let center_to_1 = measured_line.1 - center;

                let correspondence_0_measured = measured_line.0;
                let correspondence_0_reference = if center_to_0 == Vector2::zeros() {
                    point![center.x + radius, center.y]
                } else {
                    center + center_to_0.normalize() * radius
                };

                let correspondence_1_measured = measured_line.1;
                let correspondence_1_reference = if center_to_1 == Vector2::zeros() {
                    point![center.x + radius, center.y]
                } else {
                    center + center_to_1.normalize() * radius
                };

                let measured_direction = (measured_line.0 - measured_line.1).normalize();
                let center_vector =
                    (correspondence_0_reference - center) + (correspondence_1_reference - center);
                let center_vector_rotated_by_90_degree = vector![-center_vector.y, center_vector.x];
                let reference_direction = center_vector_rotated_by_90_degree.normalize();

                Correspondences {
                    correspondence_points: (
                        CorrespondencePoints {
                            measured: correspondence_0_measured,
                            reference: correspondence_0_reference,
                        },
                        CorrespondencePoints {
                            measured: correspondence_1_measured,
                            reference: correspondence_1_reference,
                        },
                    ),
                    measured_direction,
                    reference_direction,
                }
            }
        }
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
pub struct Correspondences {
    pub correspondence_points: (CorrespondencePoints, CorrespondencePoints),
    pub measured_direction: Vector2<f32>,
    pub reference_direction: Vector2<f32>,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
pub struct CorrespondencePoints {
    pub measured: Point2<f32>,
    pub reference: Point2<f32>,
}

pub fn field_marks_from_field_dimensions(field_dimensions: &FieldDimensions) -> Vec<FieldMark> {
    vec![
        FieldMark::Line {
            line: Line(
                point![-field_dimensions.length / 2.0, field_dimensions.width / 2.0],
                point![field_dimensions.length / 2.0, field_dimensions.width / 2.0],
            ),
            direction: Direction::PositiveX,
        },
        FieldMark::Line {
            line: Line(
                point![
                    -field_dimensions.length / 2.0,
                    -field_dimensions.width / 2.0
                ],
                point![field_dimensions.length / 2.0, -field_dimensions.width / 2.0],
            ),
            direction: Direction::PositiveX,
        },
        FieldMark::Line {
            line: Line(
                point![
                    -field_dimensions.length / 2.0,
                    -field_dimensions.width / 2.0
                ],
                point![-field_dimensions.length / 2.0, field_dimensions.width / 2.0],
            ),
            direction: Direction::PositiveY,
        },
        FieldMark::Line {
            line: Line(
                point![field_dimensions.length / 2.0, -field_dimensions.width / 2.0],
                point![field_dimensions.length / 2.0, field_dimensions.width / 2.0],
            ),
            direction: Direction::PositiveY,
        },
        FieldMark::Line {
            line: Line(
                point![
                    -field_dimensions.length / 2.0,
                    field_dimensions.penalty_area_width / 2.0
                ],
                point![
                    -field_dimensions.length / 2.0 + field_dimensions.penalty_area_length,
                    field_dimensions.penalty_area_width / 2.0
                ],
            ),
            direction: Direction::PositiveX,
        },
        FieldMark::Line {
            line: Line(
                point![
                    -field_dimensions.length / 2.0,
                    -field_dimensions.penalty_area_width / 2.0
                ],
                point![
                    -field_dimensions.length / 2.0 + field_dimensions.penalty_area_length,
                    -field_dimensions.penalty_area_width / 2.0
                ],
            ),
            direction: Direction::PositiveX,
        },
        FieldMark::Line {
            line: Line(
                point![
                    -field_dimensions.length / 2.0 + field_dimensions.penalty_area_length,
                    -field_dimensions.penalty_area_width / 2.0
                ],
                point![
                    -field_dimensions.length / 2.0 + field_dimensions.penalty_area_length,
                    field_dimensions.penalty_area_width / 2.0
                ],
            ),
            direction: Direction::PositiveY,
        },
        FieldMark::Line {
            line: Line(
                point![
                    -field_dimensions.length / 2.0,
                    field_dimensions.goal_box_area_width / 2.0
                ],
                point![
                    -field_dimensions.length / 2.0 + field_dimensions.goal_box_area_length,
                    field_dimensions.goal_box_area_width / 2.0
                ],
            ),
            direction: Direction::PositiveX,
        },
        FieldMark::Line {
            line: Line(
                point![
                    -field_dimensions.length / 2.0,
                    -field_dimensions.goal_box_area_width / 2.0
                ],
                point![
                    -field_dimensions.length / 2.0 + field_dimensions.goal_box_area_length,
                    -field_dimensions.goal_box_area_width / 2.0
                ],
            ),
            direction: Direction::PositiveX,
        },
        FieldMark::Line {
            line: Line(
                point![
                    -field_dimensions.length / 2.0 + field_dimensions.goal_box_area_length,
                    -field_dimensions.goal_box_area_width / 2.0
                ],
                point![
                    -field_dimensions.length / 2.0 + field_dimensions.goal_box_area_length,
                    field_dimensions.goal_box_area_width / 2.0
                ],
            ),
            direction: Direction::PositiveY,
        },
        FieldMark::Line {
            line: Line(
                point![
                    field_dimensions.length / 2.0 - field_dimensions.penalty_area_length,
                    field_dimensions.penalty_area_width / 2.0
                ],
                point![
                    field_dimensions.length / 2.0,
                    field_dimensions.penalty_area_width / 2.0
                ],
            ),
            direction: Direction::PositiveX,
        },
        FieldMark::Line {
            line: Line(
                point![
                    field_dimensions.length / 2.0 - field_dimensions.penalty_area_length,
                    -field_dimensions.penalty_area_width / 2.0
                ],
                point![
                    field_dimensions.length / 2.0,
                    -field_dimensions.penalty_area_width / 2.0
                ],
            ),
            direction: Direction::PositiveX,
        },
        FieldMark::Line {
            line: Line(
                point![
                    field_dimensions.length / 2.0 - field_dimensions.penalty_area_length,
                    -field_dimensions.penalty_area_width / 2.0
                ],
                point![
                    field_dimensions.length / 2.0 - field_dimensions.penalty_area_length,
                    field_dimensions.penalty_area_width / 2.0
                ],
            ),
            direction: Direction::PositiveY,
        },
        FieldMark::Line {
            line: Line(
                point![
                    field_dimensions.length / 2.0 - field_dimensions.goal_box_area_length,
                    field_dimensions.goal_box_area_width / 2.0
                ],
                point![
                    field_dimensions.length / 2.0,
                    field_dimensions.goal_box_area_width / 2.0
                ],
            ),
            direction: Direction::PositiveX,
        },
        FieldMark::Line {
            line: Line(
                point![
                    field_dimensions.length / 2.0 - field_dimensions.goal_box_area_length,
                    -field_dimensions.goal_box_area_width / 2.0
                ],
                point![
                    field_dimensions.length / 2.0,
                    -field_dimensions.goal_box_area_width / 2.0
                ],
            ),
            direction: Direction::PositiveX,
        },
        FieldMark::Line {
            line: Line(
                point![
                    field_dimensions.length / 2.0 - field_dimensions.goal_box_area_length,
                    -field_dimensions.goal_box_area_width / 2.0
                ],
                point![
                    field_dimensions.length / 2.0 - field_dimensions.goal_box_area_length,
                    field_dimensions.goal_box_area_width / 2.0
                ],
            ),
            direction: Direction::PositiveY,
        },
        FieldMark::Line {
            line: Line(
                point![0.0, -field_dimensions.width / 2.0],
                point![0.0, field_dimensions.width / 2.0],
            ),
            direction: Direction::PositiveY,
        },
        FieldMark::Circle {
            center: Point2::origin(),
            radius: field_dimensions.center_circle_diameter / 2.0,
        },
        FieldMark::Line {
            line: Line(
                point![
                    -field_dimensions.length / 2.0 + field_dimensions.penalty_marker_distance
                        - field_dimensions.penalty_marker_size / 2.0,
                    0.0
                ],
                point![
                    -field_dimensions.length / 2.0
                        + field_dimensions.penalty_marker_distance
                        + field_dimensions.penalty_marker_size / 2.0,
                    0.0
                ],
            ),
            direction: Direction::PositiveX,
        },
        FieldMark::Line {
            line: Line(
                point![
                    -field_dimensions.length / 2.0 + field_dimensions.penalty_marker_distance,
                    -field_dimensions.penalty_marker_size / 2.0
                ],
                point![
                    -field_dimensions.length / 2.0 + field_dimensions.penalty_marker_distance,
                    field_dimensions.penalty_marker_size / 2.0
                ],
            ),
            direction: Direction::PositiveY,
        },
        FieldMark::Line {
            line: Line(
                point![
                    field_dimensions.length / 2.0
                        - field_dimensions.penalty_marker_distance
                        - field_dimensions.penalty_marker_size / 2.0,
                    0.0
                ],
                point![
                    field_dimensions.length / 2.0 - field_dimensions.penalty_marker_distance
                        + field_dimensions.penalty_marker_size / 2.0,
                    0.0
                ],
            ),
            direction: Direction::PositiveX,
        },
        FieldMark::Line {
            line: Line(
                point![
                    field_dimensions.length / 2.0 - field_dimensions.penalty_marker_distance,
                    -field_dimensions.penalty_marker_size / 2.0
                ],
                point![
                    field_dimensions.length / 2.0 - field_dimensions.penalty_marker_distance,
                    field_dimensions.penalty_marker_size / 2.0
                ],
            ),
            direction: Direction::PositiveY,
        },
    ]
}
