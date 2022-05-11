use std::f32::consts::{FRAC_PI_8, PI};

use anyhow::{Context, Result};
use macros::{module, require_some, SerializeHierarchy};
use nalgebra::{point, vector, Isometry2, Matrix, Point, Rotation2, SMatrix, Vector2, Vector3};
use ordered_float::NotNan;
use serde::{Deserialize, Serialize};

use crate::{
    control::filtering::PoseFilter,
    types::{FieldDimensions, InitialPose, Line, Line2, LineData, Players, PrimaryState, Side},
};

#[derive(Default, Clone, Debug, Serialize, Deserialize, SerializeHierarchy)]
pub struct PoseEstimation {
    hypotheses: Vec<PoseFilter>,
}

#[module(control)]
#[input(path = current_odometry_to_last_odometry, data_type = Isometry2<f32>)]
#[input(path = primary_state, data_type = PrimaryState)]
#[perception_input(name = line_data_top, path = line_data, data_type = LineData, cycler = vision_top)]
#[perception_input(name = line_data_bottom, path = line_data, data_type = LineData, cycler = vision_bottom)]
#[additional_output(path = pose_estimation, data_type = PoseEstimation)]
#[parameter(path = field_dimensions, data_type = FieldDimensions)]
#[parameter(path = control.pose_estimation.line_measurement_noise, data_type = Vector2<f32>)]
#[parameter(path = control.pose_estimation.odometry_noise, data_type = Vector3<f32>)]
#[parameter(path = control.pose_estimation.minimal_line_length, data_type = f32)]
#[parameter(path = control.pose_estimation.angle_similarity_threshold, data_type = f32)]
#[parameter(path = control.pose_estimation.maximum_association_distance, data_type = f32)]
#[parameter(path = control.pose_estimation.use_line_measurements, data_type = bool)]
#[parameter(path = control.pose_estimation.maximum_line_distance, data_type = f32)]
#[parameter(path = control.pose_estimation.initial_poses, data_type = Players<InitialPose>)]
#[parameter(path = player_number, data_type = usize)]
#[main_output(name = robot_to_field, data_type = Isometry2<f32>)]
impl PoseEstimation {}

impl PoseEstimation {
    pub fn new() -> Self {
        Self {
            hypotheses: vec![PoseFilter::new(
                vector![-3.2, -3.0, PI / 2.0],
                0.001 * SMatrix::<f32, 3, 3>::identity(),
                10.0,
            )],
        }
    }

    fn cycle(&mut self, mut context: CycleContext) -> Result<MainOutputs> {
        let current_odometry_to_last_odometry =
            require_some!(context.current_odometry_to_last_odometry);
        let primary_state = require_some!(context.primary_state);
        match primary_state {
            PrimaryState::Unstiff => (),
            PrimaryState::Initial => {
                let initial_pose = generate_initial_isometry2(
                    &context.initial_poses[*context.player_number],
                    context.field_dimensions,
                );
                self.hypotheses = vec![PoseFilter::new(
                    vector![
                        initial_pose.translation.x,
                        initial_pose.translation.y,
                        initial_pose.rotation.angle()
                    ],
                    0.001 * SMatrix::<f32, 3, 3>::identity(),
                    10.0,
                )];
            }
            PrimaryState::Ready => (),
            PrimaryState::Set => (),
            PrimaryState::Playing => (),
            PrimaryState::Penalized => {
                let penalized_poses = vec![
                    Isometry2::new(
                        vector!(
                            -context.field_dimensions.length * 0.5
                                + context.field_dimensions.penalty_marker_distance,
                            -context.field_dimensions.width * 0.5
                        ),
                        std::f32::consts::FRAC_PI_2,
                    ),
                    Isometry2::new(
                        vector!(
                            -context.field_dimensions.length * 0.5
                                + context.field_dimensions.penalty_marker_distance,
                            context.field_dimensions.width * 0.5
                        ),
                        -std::f32::consts::FRAC_PI_2,
                    ),
                ];
                let penalized_filters = penalized_poses
                    .iter()
                    .map(|pose| {
                        PoseFilter::new(
                            vector![
                                pose.translation.x,
                                pose.translation.y,
                                pose.rotation.angle()
                            ],
                            0.001 * SMatrix::<f32, 3, 3>::identity(),
                            10.0,
                        )
                    })
                    .collect();
                self.hypotheses = penalized_filters;
            }
            PrimaryState::Finished => (),
            PrimaryState::Calibration => (),
        }

        let field_lines = generate_field_lines(context.field_dimensions);

        let line_datas = context
            .line_data_top
            .persistent
            .values()
            .zip(context.line_data_bottom.persistent.values());
        for (line_datas_top, line_datas_bottom) in line_datas {
            for filter in &mut self.hypotheses {
                // predict
                // this is knowingly using the odometry of the current cycle representatively for all
                // cycles. Fix after GORE
                predict(
                    filter,
                    current_odometry_to_last_odometry,
                    context.odometry_noise,
                )?;

                if *context.use_line_measurements {
                    let lines: Vec<_> = line_datas_top
                        .iter()
                        .chain(line_datas_bottom.iter())
                        .filter_map(|&data| data.as_ref())
                        .flat_map(|line_data| line_data.lines_in_robot.iter())
                        .filter(|line| {
                            let distance_to_robot = line.distance_to_point(Point::origin());
                            line.length() > *context.minimal_line_length
                                && distance_to_robot < *context.maximum_line_distance
                        })
                        .collect();
                    let score = update_with_lines(
                        filter,
                        &lines,
                        &field_lines,
                        context.line_measurement_noise,
                        *context.angle_similarity_threshold,
                        *context.maximum_association_distance,
                    )
                    .context("Failed to update with lines")?;
                    filter.add_score(score);
                }
            }
        }
        self.hypotheses
            .sort_by_key(|filter| NotNan::new(-filter.score()).expect("filter score was NaN"));
        let best_hypothesis = self
            .hypotheses
            .first()
            .expect("There is always at least one hypothesis in the filter");
        let best_score = best_hypothesis.score();
        let robot_to_field = best_hypothesis.isometry();
        self.hypotheses
            .retain(|filter| filter.score() >= 0.1 * best_score);

        context.pose_estimation.on_subscription(|| self.clone());
        Ok(MainOutputs {
            robot_to_field: Some(robot_to_field),
        })
    }
}

fn predict(
    filter: &mut PoseFilter,
    current_odometry_to_last_odometry: &Isometry2<f32>,
    odometry_noise: &Vector3<f32>,
) -> Result<()> {
    let current_orientation_angle = filter.state_mean().z;
    // rotate odometry noise from robot frame to field frame
    let rotated_noise = Rotation2::new(current_orientation_angle) * odometry_noise.xy();
    let process_noise = Matrix::from_diagonal(&vector![
        rotated_noise.x.abs(),
        rotated_noise.y.abs(),
        odometry_noise.z
    ]);
    filter.predict(
        |state| {
            // rotate odometry from robot frame to field frame
            let robot_odometry =
                Rotation2::new(state.z) * current_odometry_to_last_odometry.translation.vector;
            vector![
                state.x + robot_odometry.x,
                state.y + robot_odometry.y,
                state.z + current_odometry_to_last_odometry.rotation.angle()
            ]
        },
        process_noise,
    )
}

fn update_with_lines(
    filter: &mut PoseFilter,
    measured_lines: &[&Line2],
    field_lines: &[(LineType, Line2)],
    line_measurement_noise: &Vector2<f32>,
    angle_similarity_threshold: f32,
    maximum_association_distance: f32,
) -> Result<f32> {
    let robot_to_field = filter.isometry();

    for line in measured_lines {
        update_with_line(
            filter,
            line,
            line_measurement_noise,
            robot_to_field,
            field_lines,
            angle_similarity_threshold,
            maximum_association_distance,
        )
        .context("Failed to update with line")?
    }

    let score = score_with_line_association(
        filter,
        robot_to_field,
        measured_lines,
        field_lines,
        angle_similarity_threshold,
        maximum_association_distance,
    );
    Ok(score)
}

fn score_with_line_association(
    filter: &PoseFilter,
    robot_to_field: Isometry2<f32>,
    measured_lines: &[&Line2],
    field_lines: &[(LineType, Line2)],
    angle_similarity_threshold: f32,
    maximum_association_distance: f32,
) -> f32 {
    measured_lines
        .iter()
        .filter(|&&&measured_line| {
            let line_in_field = robot_to_field * measured_line;
            find_closest_line(
                line_in_field,
                &filter.state_covariance(),
                field_lines,
                angle_similarity_threshold,
                maximum_association_distance,
            )
            .is_some()
        })
        .count() as f32
}

fn update_with_line(
    filter: &mut PoseFilter,
    &measured_line: &Line2,
    measurement_noise: &Vector2<f32>,
    robot_to_field: Isometry2<f32>,
    field_lines: &[(LineType, Line2)],
    angle_similarity_threshold: f32,
    maximum_association_distance: f32,
) -> Result<()> {
    let line_in_field = robot_to_field * measured_line;
    let covariance = filter.state_covariance();
    let &(direction, closest_line) = match find_closest_line(
        line_in_field,
        &covariance,
        field_lines,
        angle_similarity_threshold,
        maximum_association_distance,
    ) {
        Some(matching) => matching,
        None => return Ok(()),
    };
    if let LineType::CenterCircle = direction {
        // skipping updates for lines associated with the center circle
        return Ok(());
    }
    let measurement = pose_from_line(&measured_line, &line_in_field, &closest_line, direction);
    let distance_to_measured_line = measured_line.squared_distance_to_segment(Point::origin());
    let measurement_noise = distance_to_measured_line.max(1.0) * measurement_noise;

    filter
        .update(
            measurement,
            Matrix::from_diagonal(&measurement_noise),
            |state| match direction {
                LineType::AlongX => vector![state.y, state.z],
                LineType::AlongY => vector![state.x, state.z],
                LineType::CenterCircle => {
                    panic!("Cannot update pose filter with a center circle line")
                }
            },
        )
        .context("Filter update failed")
}

fn pose_from_line(
    &measured_line: &Line2,
    absolute_measured_line: &Line2,
    field_line: &Line2,
    direction: LineType,
) -> Vector2<f32> {
    let distance_to_line = measured_line.distance_to_point(point![0.0, 0.0]);
    let line_sign_right = match direction {
        LineType::AlongX if absolute_measured_line.1.x < absolute_measured_line.0.x => {
            Line(measured_line.1, measured_line.0)
        }
        LineType::AlongY if absolute_measured_line.1.y < absolute_measured_line.0.y => {
            Line(measured_line.1, measured_line.0)
        }
        _ => measured_line,
    };
    let line_direction = line_sign_right.1 - line_sign_right.0;
    let is_left_of_line = line_sign_right.0.coords.dot(&line_sign_right.1.coords) < 0.0;
    let signed_distance = distance_to_line * if is_left_of_line { 1.0 } else { -1.0 };
    match direction {
        LineType::AlongX => vector![
            field_line.0.y + signed_distance,
            (-line_direction.y).atan2(line_direction.x)
        ],
        LineType::AlongY => vector![
            field_line.0.x - signed_distance,
            line_direction.x.atan2(line_direction.y)
        ],
        LineType::CenterCircle => panic!("Cannot update pose filter with a center circle line"),
    }
}

fn find_closest_line<'a>(
    line_in_field: Line2,
    covariance: &SMatrix<f32, 3, 3>,
    field_lines: &'a [(LineType, Line2)],
    angle_similarity_threshold: f32,
    maximum_association_distance: f32,
) -> Option<&'a (LineType, Line2)> {
    let line_direction = (line_in_field.1 - line_in_field.0).normalize();
    let measured_line_center = Point::from((line_in_field.0.coords + line_in_field.1.coords) / 2.0);
    let variance = covariance.diagonal();
    let angle_similarity_threshold = angle_similarity_threshold + variance.z.sqrt();
    let measured_line_length = line_in_field.length();
    let distance_threshold = maximum_association_distance + variance.x.max(variance.y).sqrt();

    field_lines
        .iter()
        .filter(|(_, candidate)| {
            let candidate_direction = (candidate.1 - candidate.0).normalize();
            let angle_between_lines = line_direction.dot(&candidate_direction).abs().acos();
            let center_distance = candidate
                .squared_distance_to_segment(measured_line_center)
                .sqrt();
            let candidate_line_length = candidate.length();
            angle_between_lines < angle_similarity_threshold
                && center_distance < distance_threshold
                && measured_line_length < candidate_line_length
        })
        .min_by_key(|(_, candidate)| {
            let center_distance = candidate.squared_distance_to_segment(measured_line_center);
            NotNan::new(center_distance)
                .context("Distance to line segment was NaN")
                .unwrap()
        })
}

#[derive(Clone, Copy, Debug)]
enum LineType {
    AlongX,
    AlongY,
    CenterCircle,
}

fn generate_field_lines(field_dimensions: &FieldDimensions) -> Vec<(LineType, Line2)> {
    let field_length = field_dimensions.length;
    let field_width = field_dimensions.width;
    let penalty_area_length = field_dimensions.penalty_area_length;
    let penalty_area_width = field_dimensions.penalty_area_width;
    let goal_box_area_length = field_dimensions.goal_box_area_length;
    let goal_box_area_width = field_dimensions.goal_box_area_width;
    let center_circle_radius = field_dimensions.center_circle_diameter / 2.0;
    let goal_post_distance =
        field_dimensions.goal_inner_width + field_dimensions.goal_post_diameter;
    let goal_depth = field_dimensions.goal_depth;

    let penalty_box_distance = field_length / 2.0 - penalty_area_length;
    let goal_box_area_distance = field_length / 2.0 - goal_box_area_length;

    let mut lines = vec![
        // field border
        (
            LineType::AlongX,
            Line(
                point![-field_length / 2.0, field_width / 2.0],
                point![field_length / 2.0, field_width / 2.0],
            ),
        ),
        (
            LineType::AlongX,
            Line(
                point![-field_length / 2.0, -field_width / 2.0],
                point![field_length / 2.0, -field_width / 2.0],
            ),
        ),
        (
            LineType::AlongY,
            Line(
                point![-field_length / 2.0, field_width / 2.0],
                point![-field_length / 2.0, -field_width / 2.0],
            ),
        ),
        (
            LineType::AlongY,
            Line(
                point![field_length / 2.0, field_width / 2.0],
                point![field_length / 2.0, -field_width / 2.0],
            ),
        ),
        // center line
        (
            LineType::AlongY,
            Line(
                point![0.0, field_width / 2.0],
                point![0.0, -field_width / 2.0],
            ),
        ),
        // penalty box home
        (
            LineType::AlongX,
            Line(
                point![-field_length / 2.0, penalty_area_width / 2.0],
                point![-penalty_box_distance, penalty_area_width / 2.0],
            ),
        ),
        (
            LineType::AlongX,
            Line(
                point![-field_length / 2.0, -penalty_area_width / 2.0],
                point![-penalty_box_distance, -penalty_area_width / 2.0],
            ),
        ),
        (
            LineType::AlongY,
            Line(
                point![-penalty_box_distance, penalty_area_width / 2.0],
                point![-penalty_box_distance, -penalty_area_width / 2.0],
            ),
        ),
        // penalty box away
        (
            LineType::AlongX,
            Line(
                point![field_length / 2.0, penalty_area_width / 2.0],
                point![penalty_box_distance, penalty_area_width / 2.0],
            ),
        ),
        (
            LineType::AlongX,
            Line(
                point![field_length / 2.0, -penalty_area_width / 2.0],
                point![penalty_box_distance, -penalty_area_width / 2.0],
            ),
        ),
        (
            LineType::AlongY,
            Line(
                point![penalty_box_distance, penalty_area_width / 2.0],
                point![penalty_box_distance, -penalty_area_width / 2.0],
            ),
        ),
        // goal box area home
        (
            LineType::AlongX,
            Line(
                point![-field_length / 2.0, goal_box_area_width / 2.0],
                point![-goal_box_area_distance, goal_box_area_width / 2.0],
            ),
        ),
        (
            LineType::AlongX,
            Line(
                point![-field_length / 2.0, -goal_box_area_width / 2.0],
                point![-goal_box_area_distance, -goal_box_area_width / 2.0],
            ),
        ),
        (
            LineType::AlongY,
            Line(
                point![-goal_box_area_distance, goal_box_area_width / 2.0],
                point![-goal_box_area_distance, -goal_box_area_width / 2.0],
            ),
        ),
        // goal box area away
        (
            LineType::AlongX,
            Line(
                point![field_length / 2.0, goal_box_area_width / 2.0],
                point![goal_box_area_distance, goal_box_area_width / 2.0],
            ),
        ),
        (
            LineType::AlongX,
            Line(
                point![field_length / 2.0, -goal_box_area_width / 2.0],
                point![goal_box_area_distance, -goal_box_area_width / 2.0],
            ),
        ),
        (
            LineType::AlongY,
            Line(
                point![goal_box_area_distance, goal_box_area_width / 2.0],
                point![goal_box_area_distance, -goal_box_area_width / 2.0],
            ),
        ),
        // goal support structure opponent
        (
            LineType::AlongX,
            Line(
                point![field_length / 2.0, -goal_post_distance / 2.0],
                point![field_length / 2.0 + goal_depth, -goal_post_distance / 2.0],
            ),
        ),
        (
            LineType::AlongX,
            Line(
                point![field_length / 2.0, goal_post_distance / 2.0],
                point![field_length / 2.0 + goal_depth, goal_post_distance / 2.0],
            ),
        ),
        (
            LineType::AlongY,
            Line(
                point![field_length / 2.0 + goal_depth, -goal_post_distance / 2.0],
                point![field_length / 2.0 + goal_depth, goal_post_distance / 2.0],
            ),
        ),
        // goal support structure own
        (
            LineType::AlongX,
            Line(
                point![-field_length / 2.0, -goal_post_distance / 2.0],
                point![-field_length / 2.0 - goal_depth, -goal_post_distance / 2.0],
            ),
        ),
        (
            LineType::AlongX,
            Line(
                point![-field_length / 2.0, goal_post_distance / 2.0],
                point![-field_length / 2.0 - goal_depth, goal_post_distance / 2.0],
            ),
        ),
        (
            LineType::AlongY,
            Line(
                point![-field_length / 2.0 - goal_depth, -goal_post_distance / 2.0],
                point![-field_length / 2.0 - goal_depth, goal_post_distance / 2.0],
            ),
        ),
    ];
    // center circle as polygon
    for i in 0..8 {
        let p1 = Rotation2::new(i as f32 * FRAC_PI_8) * point![center_circle_radius, 0.0];
        let p2 = Rotation2::new((i + 1) as f32 * FRAC_PI_8) * point![center_circle_radius, 0.0];
        lines.push((LineType::CenterCircle, Line(p1, p2)));
    }
    lines
}

fn generate_initial_isometry2(
    initial_pose: &InitialPose,
    field_dimensions: &FieldDimensions,
) -> Isometry2<f32> {
    match initial_pose.side {
        Side::Left => Isometry2::new(
            vector!(
                initial_pose.center_line_offset_x,
                field_dimensions.width * 0.5
            ),
            -std::f32::consts::FRAC_PI_2,
        ),
        Side::Right => Isometry2::new(
            vector!(
                initial_pose.center_line_offset_x,
                -field_dimensions.width * 0.5
            ),
            std::f32::consts::FRAC_PI_2,
        ),
    }
}
