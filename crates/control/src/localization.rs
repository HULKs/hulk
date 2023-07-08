use std::{
    f32::consts::{FRAC_PI_2, PI},
    mem::take,
};

use approx::assert_relative_eq;
use color_eyre::{eyre::WrapErr, Result};
use context_attribute::context;
use filtering::pose_filter::PoseFilter;
use framework::{AdditionalOutput, HistoricInput, MainOutput, PerceptionInput};
use nalgebra::{
    distance, matrix, point, vector, Isometry2, Matrix, Matrix2, Matrix3, Point2, Rotation2,
    Translation2, Vector2, Vector3,
};
use ordered_float::NotNan;
use spl_network_messages::{GamePhase, Penalty, PlayerNumber, Team};
use types::{
    field_marks_from_field_dimensions,
    localization::{ScoredPose, Update},
    multivariate_normal_distribution::MultivariateNormalDistribution,
    CorrespondencePoints, Direction, FieldDimensions, FieldMark, GameControllerState, InitialPose,
    Line, Line2, LineData, Players, PrimaryState, Side,
};

pub struct Localization {
    field_marks: Vec<FieldMark>,
    last_primary_state: PrimaryState,
    hypotheses: Vec<ScoredPose>,
    hypotheses_when_entered_playing: Vec<ScoredPose>,
    is_penalized_with_motion_in_set: bool,
    was_picked_up_while_penalized_with_motion_in_set: bool,
}

#[context]
pub struct CreationContext {
    pub field_dimensions: Parameter<FieldDimensions, "field_dimensions">,
}

#[context]
pub struct CycleContext {
    pub correspondence_lines: AdditionalOutput<Vec<Line2>, "localization.correspondence_lines">,
    pub fit_errors: AdditionalOutput<Vec<Vec<Vec<Vec<f32>>>>, "localization.fit_errors">,
    pub measured_lines_in_field:
        AdditionalOutput<Vec<Line2>, "localization.measured_lines_in_field">,
    pub pose_hypotheses: AdditionalOutput<Vec<ScoredPose>, "localization.pose_hypotheses">,
    pub updates: AdditionalOutput<Vec<Vec<Update>>, "localization.updates">,

    pub current_odometry_to_last_odometry:
        HistoricInput<Option<Isometry2<f32>>, "current_odometry_to_last_odometry?">,

    pub game_controller_state: Input<Option<GameControllerState>, "game_controller_state?">,
    pub has_ground_contact: Input<bool, "has_ground_contact">,
    pub primary_state: Input<PrimaryState, "primary_state">,

    pub circle_measurement_noise: Parameter<Vector2<f32>, "localization.circle_measurement_noise">,
    pub field_dimensions: Parameter<FieldDimensions, "field_dimensions">,
    pub good_matching_threshold: Parameter<f32, "localization.good_matching_threshold">,
    pub gradient_convergence_threshold:
        Parameter<f32, "localization.gradient_convergence_threshold">,
    pub gradient_descent_step_size: Parameter<f32, "localization.gradient_descent_step_size">,
    pub hypothesis_prediction_score_reduction_factor:
        Parameter<f32, "localization.hypothesis_prediction_score_reduction_factor">,
    pub hypothesis_retain_factor: Parameter<f32, "localization.hypothesis_retain_factor">,
    pub hypothesis_score_base_increase:
        Parameter<f32, "localization.hypothesis_score_base_increase">,
    pub initial_hypothesis_covariance:
        Parameter<Matrix3<f32>, "localization.initial_hypothesis_covariance">,
    pub initial_hypothesis_score: Parameter<f32, "localization.initial_hypothesis_score">,
    pub initial_poses: Parameter<Players<InitialPose>, "localization.initial_poses">,
    pub line_length_acceptance_factor: Parameter<f32, "localization.line_length_acceptance_factor">,
    pub line_measurement_noise: Parameter<Vector2<f32>, "localization.line_measurement_noise">,
    pub maximum_amount_of_gradient_descent_iterations:
        Parameter<usize, "localization.maximum_amount_of_gradient_descent_iterations">,
    pub maximum_amount_of_outer_iterations:
        Parameter<usize, "localization.maximum_amount_of_outer_iterations">,
    pub minimum_fit_error: Parameter<f32, "localization.minimum_fit_error">,
    pub odometry_noise: Parameter<Vector3<f32>, "localization.odometry_noise">,
    pub player_number: Parameter<PlayerNumber, "player_number">,
    pub score_per_good_match: Parameter<f32, "localization.score_per_good_match">,
    pub use_line_measurements: Parameter<bool, "localization.use_line_measurements">,
    pub injected_robot_to_field_of_home_after_coin_toss_before_second_half: Parameter<
        Option<Isometry2<f32>>,
        "injected_robot_to_field_of_home_after_coin_toss_before_second_half?",
    >,

    pub line_data_bottom: PerceptionInput<Option<LineData>, "VisionBottom", "line_data?">,
    pub line_data_top: PerceptionInput<Option<LineData>, "VisionTop", "line_data?">,

    pub robot_to_field: PersistentState<Isometry2<f32>, "robot_to_field">,
}

#[context]
#[derive(Default)]
pub struct MainOutputs {
    pub robot_to_field: MainOutput<Option<Isometry2<f32>>>,
    pub robot_to_field_of_home_after_coin_toss_before_second_half:
        MainOutput<Option<Isometry2<f32>>>,
}

impl Localization {
    pub fn new(context: CreationContext) -> Result<Self> {
        Ok(Self {
            field_marks: field_marks_from_field_dimensions(context.field_dimensions)
                .into_iter()
                .chain(goal_support_structure_line_marks_from_field_dimensions(
                    context.field_dimensions,
                ))
                .collect(),
            last_primary_state: PrimaryState::Unstiff,
            hypotheses: vec![],
            hypotheses_when_entered_playing: vec![],
            is_penalized_with_motion_in_set: false,
            was_picked_up_while_penalized_with_motion_in_set: false,
        })
    }

    fn reset_state(
        &mut self,
        primary_state: PrimaryState,
        game_phase: Option<GamePhase>,
        context: &CycleContext,
        penalty: &Option<Penalty>,
    ) {
        match (self.last_primary_state, primary_state, game_phase) {
            (PrimaryState::Initial, PrimaryState::Ready, _) => {
                let initial_pose = generate_initial_pose(
                    &context.initial_poses[*context.player_number],
                    context.field_dimensions,
                );
                self.hypotheses = vec![ScoredPose::from_isometry(
                    initial_pose,
                    *context.initial_hypothesis_covariance,
                    *context.initial_hypothesis_score,
                )];
                self.hypotheses_when_entered_playing = self.hypotheses.clone();
            }
            (
                PrimaryState::Set,
                PrimaryState::Playing,
                Some(GamePhase::PenaltyShootout {
                    kicking_team: Team::Hulks,
                }),
            ) => {
                let penalty_shoot_out_striker_pose = Isometry2::translation(
                    -context.field_dimensions.penalty_area_length
                        + (context.field_dimensions.length / 2.0),
                    0.0,
                );
                self.hypotheses = vec![ScoredPose::from_isometry(
                    penalty_shoot_out_striker_pose,
                    *context.initial_hypothesis_covariance,
                    *context.initial_hypothesis_score,
                )];
                self.hypotheses_when_entered_playing = self.hypotheses.clone();
            }
            (
                PrimaryState::Set,
                PrimaryState::Playing,
                Some(GamePhase::PenaltyShootout {
                    kicking_team: Team::Opponent,
                }),
            ) => {
                let penalty_shoot_out_keeper_pose =
                    Isometry2::translation(-context.field_dimensions.length / 2.0, 0.0);
                self.hypotheses = vec![ScoredPose::from_isometry(
                    penalty_shoot_out_keeper_pose,
                    *context.initial_hypothesis_covariance,
                    *context.initial_hypothesis_score,
                )];
                self.hypotheses_when_entered_playing = self.hypotheses.clone();
            }
            (PrimaryState::Set, PrimaryState::Playing, _) => {
                self.hypotheses_when_entered_playing = self.hypotheses.clone();
            }
            (PrimaryState::Playing, PrimaryState::Penalized, _) => {
                match penalty {
                    Some(Penalty::IllegalMotionInSet { remaining: _ }) => {
                        self.is_penalized_with_motion_in_set = true;
                    }
                    Some(_) => {}
                    None => {}
                };
            }
            (PrimaryState::Penalized, _, _) if primary_state != PrimaryState::Penalized => {
                if self.is_penalized_with_motion_in_set {
                    if self.was_picked_up_while_penalized_with_motion_in_set {
                        self.hypotheses = take(&mut self.hypotheses_when_entered_playing);

                        let penalized_poses = generate_penalized_poses(context.field_dimensions);
                        self.hypotheses_when_entered_playing = penalized_poses
                            .into_iter()
                            .map(|pose| {
                                ScoredPose::from_isometry(
                                    pose,
                                    *context.initial_hypothesis_covariance,
                                    *context.initial_hypothesis_score,
                                )
                            })
                            .collect();
                    }
                    self.is_penalized_with_motion_in_set = false;
                    self.was_picked_up_while_penalized_with_motion_in_set = false;
                } else {
                    let penalized_poses = generate_penalized_poses(context.field_dimensions);
                    self.hypotheses = penalized_poses
                        .into_iter()
                        .map(|pose| {
                            ScoredPose::from_isometry(
                                pose,
                                *context.initial_hypothesis_covariance,
                                *context.initial_hypothesis_score,
                            )
                        })
                        .collect();
                    self.hypotheses_when_entered_playing = self.hypotheses.clone();
                }
            }
            (PrimaryState::Unstiff, _, _) => {
                let penalized_poses = generate_penalized_poses(context.field_dimensions);
                self.hypotheses = penalized_poses
                    .into_iter()
                    .map(|pose| {
                        ScoredPose::from_isometry(
                            pose,
                            *context.initial_hypothesis_covariance,
                            *context.initial_hypothesis_score,
                        )
                    })
                    .collect();
                self.hypotheses_when_entered_playing = self.hypotheses.clone();
            }
            _ => {}
        }
    }

    fn update_state(&mut self, context: &mut CycleContext) -> Result<()> {
        let mut fit_errors_per_measurement = vec![];

        context.measured_lines_in_field.fill_if_subscribed(Vec::new);
        context.correspondence_lines.fill_if_subscribed(Vec::new);
        context
            .updates
            .fill_if_subscribed(|| vec![vec![]; self.hypotheses.len()]);

        let line_datas = context
            .line_data_top
            .persistent
            .iter()
            .zip(context.line_data_bottom.persistent.iter());
        for (
            (line_data_top_timestamp, line_data_top),
            (line_data_bottom_timestamp, line_data_bottom),
        ) in line_datas
        {
            assert_eq!(line_data_top_timestamp, line_data_bottom_timestamp);
            let current_odometry_to_last_odometry = context
                .current_odometry_to_last_odometry
                .get(line_data_top_timestamp);

            let mut fit_errors_per_hypothesis = vec![];
            for (hypothesis_index, scored_state) in self.hypotheses.iter_mut().enumerate() {
                if let Some(current_odometry_to_last_odometry) = current_odometry_to_last_odometry {
                    predict(
                        &mut scored_state.state,
                        current_odometry_to_last_odometry,
                        context.odometry_noise,
                    )
                    .wrap_err("failed to predict pose filter")?;
                    scored_state.score *= *context.hypothesis_prediction_score_reduction_factor;
                }
                if *context.use_line_measurements {
                    let robot_to_field = scored_state.state.as_isometry();
                    let current_measured_lines_in_field: Vec<_> = line_data_top
                        .iter()
                        .chain(line_data_bottom.iter())
                        .filter_map(|data| data.as_ref())
                        .flat_map(|line_data| {
                            line_data
                                .lines_in_robot
                                .iter()
                                .map(|&measured_line_in_robot| {
                                    robot_to_field * measured_line_in_robot
                                })
                        })
                        .collect();
                    context.measured_lines_in_field.mutate_if_subscribed(
                        |measured_lines_in_field| {
                            if let Some(measured_lines_in_field) = measured_lines_in_field {
                                measured_lines_in_field
                                    .extend(current_measured_lines_in_field.iter());
                            }
                        },
                    );
                    if current_measured_lines_in_field.is_empty() {
                        continue;
                    }

                    let (field_mark_correspondences, fit_error, fit_errors) =
                        get_fitted_field_mark_correspondence(
                            &current_measured_lines_in_field,
                            &self.field_marks,
                            *context.gradient_convergence_threshold,
                            *context.gradient_descent_step_size,
                            *context.line_length_acceptance_factor,
                            *context.maximum_amount_of_gradient_descent_iterations,
                            *context.maximum_amount_of_outer_iterations,
                            context.fit_errors.is_subscribed(),
                        );
                    context
                        .correspondence_lines
                        .mutate_if_subscribed(|correspondence_lines| {
                            let next_correspondence_lines = field_mark_correspondences
                                .iter()
                                .flat_map(|field_mark_correspondence| {
                                    let correspondence_points_0 =
                                        field_mark_correspondence.correspondence_points.0;
                                    let correspondence_points_1 =
                                        field_mark_correspondence.correspondence_points.1;
                                    [
                                        Line(
                                            correspondence_points_0.measured,
                                            correspondence_points_0.reference,
                                        ),
                                        Line(
                                            correspondence_points_1.measured,
                                            correspondence_points_1.reference,
                                        ),
                                    ]
                                });
                            if let Some(correspondence_lines) = correspondence_lines {
                                correspondence_lines.extend(next_correspondence_lines);
                            }
                        });
                    if context.fit_errors.is_subscribed() {
                        fit_errors_per_hypothesis.push(fit_errors);
                    }
                    let clamped_fit_error = fit_error.max(*context.minimum_fit_error);
                    let number_of_measurements_weight =
                        1.0 / field_mark_correspondences.len() as f32;

                    for field_mark_correspondence in field_mark_correspondences {
                        let update = match field_mark_correspondence.field_mark {
                            FieldMark::Line { .. } => get_translation_and_rotation_measurement(
                                robot_to_field,
                                field_mark_correspondence,
                            ),
                            FieldMark::Circle { .. } => get_2d_translation_measurement(
                                robot_to_field,
                                field_mark_correspondence,
                            ),
                        };
                        let line_length = field_mark_correspondence.measured_line_in_field.length();
                        let line_length_weight = if line_length == 0.0 {
                            1.0
                        } else {
                            1.0 / line_length
                        };
                        let line_center_point =
                            field_mark_correspondence.measured_line_in_field.center();
                        let line_distance_to_robot = distance(
                            &line_center_point,
                            &Point2::from(robot_to_field.translation.vector),
                        );
                        context.updates.mutate_if_subscribed(|updates| {
                            if let Some(updates) = updates {
                                updates[hypothesis_index].push({
                                    let robot_to_field = match field_mark_correspondence.field_mark
                                    {
                                        FieldMark::Line { line: _, direction } => match direction {
                                            Direction::PositiveX => Isometry2::new(
                                                vector![robot_to_field.translation.x, update.x],
                                                update.y,
                                            ),
                                            Direction::PositiveY => Isometry2::new(
                                                vector![update.x, robot_to_field.translation.y],
                                                update.y,
                                            ),
                                        },
                                        FieldMark::Circle { .. } => {
                                            Isometry2::new(update, robot_to_field.rotation.angle())
                                        }
                                    };
                                    Update {
                                        robot_to_field,
                                        line_center_point,
                                        fit_error: clamped_fit_error,
                                        number_of_measurements_weight,
                                        line_distance_to_robot,
                                        line_length_weight,
                                    }
                                });
                            }
                        });
                        let uncertainty_weight = clamped_fit_error
                            * number_of_measurements_weight
                            * line_length_weight
                            * line_distance_to_robot;
                        match field_mark_correspondence.field_mark {
                            FieldMark::Line { line: _, direction } => scored_state
                                .state
                                .update_with_1d_translation_and_rotation(
                                    update,
                                    Matrix::from_diagonal(context.line_measurement_noise)
                                        * uncertainty_weight,
                                    |state| match direction {
                                        Direction::PositiveX => {
                                            vector![state.y, state.z]
                                        }
                                        Direction::PositiveY => {
                                            vector![state.x, state.z]
                                        }
                                    },
                                )
                                .context("Failed to update pose filter")?,
                            FieldMark::Circle { .. } => scored_state
                                .state
                                .update_with_2d_translation(
                                    update,
                                    Matrix::from_diagonal(context.circle_measurement_noise)
                                        * uncertainty_weight,
                                    |state| vector![state.x, state.y],
                                )
                                .context("Failed to update pose filter")?,
                        }
                        if field_mark_correspondence.fit_error_sum()
                            < *context.good_matching_threshold
                        {
                            scored_state.score += *context.score_per_good_match;
                        }
                    }
                }
                scored_state.score += *context.hypothesis_score_base_increase;
            }

            if context.fit_errors.is_subscribed() {
                fit_errors_per_measurement.push(fit_errors_per_hypothesis);
            }
        }

        let best_hypothesis = self
            .get_best_hypothesis()
            .expect("Expected at least one hypothesis");
        let best_score = best_hypothesis.score;
        let robot_to_field = best_hypothesis.state.as_isometry();
        self.hypotheses.retain(|scored_state| {
            scored_state.score >= *context.hypothesis_retain_factor * best_score
        });

        context
            .pose_hypotheses
            .fill_if_subscribed(|| self.hypotheses.clone());
        context
            .fit_errors
            .fill_if_subscribed(|| fit_errors_per_measurement);

        *context.robot_to_field = robot_to_field;

        Ok(())
    }

    pub fn cycle(&mut self, mut context: CycleContext) -> Result<MainOutputs> {
        let primary_state = *context.primary_state;
        let penalty = context
            .game_controller_state
            .and_then(|game_controller_state| {
                game_controller_state.penalties[*context.player_number]
            });
        let game_phase = context
            .game_controller_state
            .map(|game_controller_state| game_controller_state.game_phase);

        self.reset_state(primary_state, game_phase, &context, &penalty);
        self.last_primary_state = primary_state;

        if self.is_penalized_with_motion_in_set && !context.has_ground_contact {
            self.was_picked_up_while_penalized_with_motion_in_set = true;
        }

        let robot_to_field = match primary_state {
            PrimaryState::Ready | PrimaryState::Set | PrimaryState::Playing => {
                self.update_state(&mut context)?;
                Some(*context.robot_to_field)
            }
            _ => None,
        };
        let robot_to_field_of_home_after_coin_toss_before_second_half = context
            .injected_robot_to_field_of_home_after_coin_toss_before_second_half
            .copied()
            .or_else(|| {
                robot_to_field
                    .and_then(|robot_to_field| {
                        Some((robot_to_field, context.game_controller_state?))
                    })
                    .map(|(robot_to_field, game_controller_state)| {
                        if !game_controller_state.hulks_team_is_home_after_coin_toss {
                            Isometry2::from_parts(
                                Translation2::default(),
                                Rotation2::new(PI).into(),
                            ) * robot_to_field
                        } else {
                            robot_to_field
                        }
                    })
            });
        Ok(MainOutputs {
            robot_to_field: robot_to_field.into(),
            robot_to_field_of_home_after_coin_toss_before_second_half:
                robot_to_field_of_home_after_coin_toss_before_second_half.into(),
        })
    }

    fn get_best_hypothesis(&self) -> Option<&ScoredPose> {
        self.hypotheses
            .iter()
            .max_by_key(|scored_filter| NotNan::new(scored_filter.score).unwrap())
    }
}

pub fn goal_support_structure_line_marks_from_field_dimensions(
    field_dimensions: &FieldDimensions,
) -> Vec<FieldMark> {
    let goal_width = field_dimensions.goal_inner_width + field_dimensions.goal_post_diameter;
    let goal_depth = field_dimensions.goal_depth;
    vec![
        FieldMark::Line {
            line: Line(
                point![
                    -field_dimensions.length / 2.0 - goal_depth,
                    -goal_width / 2.0
                ],
                point![
                    -field_dimensions.length / 2.0 - goal_depth,
                    goal_width / 2.0
                ],
            ),
            direction: Direction::PositiveY,
        },
        FieldMark::Line {
            line: Line(
                point![
                    -field_dimensions.length / 2.0 - goal_depth,
                    -goal_width / 2.0
                ],
                point![-field_dimensions.length / 2.0, -goal_width / 2.0],
            ),
            direction: Direction::PositiveX,
        },
        FieldMark::Line {
            line: Line(
                point![
                    -field_dimensions.length / 2.0 - goal_depth,
                    goal_width / 2.0
                ],
                point![-field_dimensions.length / 2.0, goal_width / 2.0],
            ),
            direction: Direction::PositiveX,
        },
        FieldMark::Line {
            line: Line(
                point![
                    field_dimensions.length / 2.0 + goal_depth,
                    -goal_width / 2.0
                ],
                point![field_dimensions.length / 2.0 + goal_depth, goal_width / 2.0],
            ),
            direction: Direction::PositiveY,
        },
        FieldMark::Line {
            line: Line(
                point![field_dimensions.length / 2.0, -goal_width / 2.0],
                point![
                    field_dimensions.length / 2.0 + goal_depth,
                    -goal_width / 2.0
                ],
            ),
            direction: Direction::PositiveX,
        },
        FieldMark::Line {
            line: Line(
                point![field_dimensions.length / 2.0, goal_width / 2.0],
                point![field_dimensions.length / 2.0 + goal_depth, goal_width / 2.0],
            ),
            direction: Direction::PositiveX,
        },
    ]
}

#[derive(Clone, Copy, Debug)]
pub struct FieldMarkCorrespondence {
    measured_line_in_field: Line2,
    field_mark: FieldMark,
    pub correspondence_points: (CorrespondencePoints, CorrespondencePoints),
}

impl FieldMarkCorrespondence {
    fn fit_error_sum(&self) -> f32 {
        (self.correspondence_points.0.measured - self.correspondence_points.0.reference).norm()
            + (self.correspondence_points.1.measured - self.correspondence_points.1.reference)
                .norm()
    }
}

fn predict(
    state: &mut MultivariateNormalDistribution<3>,
    current_odometry_to_last_odometry: &Isometry2<f32>,
    odometry_noise: &Vector3<f32>,
) -> Result<()> {
    let current_orientation_angle = state.mean.z;
    // rotate odometry noise from robot frame to field frame
    let rotated_noise = Rotation2::new(current_orientation_angle) * odometry_noise.xy();
    let process_noise = Matrix::from_diagonal(&vector![
        rotated_noise.x.abs(),
        rotated_noise.y.abs(),
        odometry_noise.z
    ]);

    state.predict(
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
    )?;
    Ok(())
}

pub fn get_fitted_field_mark_correspondence(
    measured_lines_in_field: &[Line2],
    field_marks: &[FieldMark],
    gradient_convergence_threshold: f32,
    gradient_descent_step_size: f32,
    line_length_acceptance_factor: f32,
    maximum_amount_of_gradient_descent_iterations: usize,
    maximum_amount_of_outer_iterations: usize,
    fit_errors_is_subscribed: bool,
) -> (Vec<FieldMarkCorrespondence>, f32, Vec<Vec<f32>>) {
    let mut fit_errors = vec![];
    let mut correction = Isometry2::identity();
    for _ in 0..maximum_amount_of_outer_iterations {
        let correspondence_points = get_correspondence_points(get_field_mark_correspondence(
            measured_lines_in_field,
            correction,
            field_marks,
            line_length_acceptance_factor,
        ));

        let weight_matrices: Vec<_> = correspondence_points
            .iter()
            .map(|correspondence_points| {
                let normal =
                    (correction * correspondence_points.measured) - correspondence_points.reference;
                if normal.norm() > 0.0 {
                    let normal_versor = normal.normalize();
                    normal_versor * normal_versor.transpose()
                } else {
                    Matrix2::zeros()
                }
            })
            .collect();

        let mut fit_errors_per_iteration = vec![];
        for _ in 0..maximum_amount_of_gradient_descent_iterations {
            let translation_gradient: Vector2<f32> = correspondence_points
                .iter()
                .zip(weight_matrices.iter())
                .map(|(correspondence_points, weight_matrix)| {
                    2.0 * weight_matrix
                        * ((correction * correspondence_points.measured)
                            - correspondence_points.reference)
                })
                .sum::<Vector2<f32>>()
                / correspondence_points.len() as f32;
            let rotation = correction.rotation.angle();
            let rotation_derivative =
                matrix![-rotation.sin(), -rotation.cos(); rotation.cos(), -rotation.sin()];
            let rotation_gradient: f32 = correspondence_points
                .iter()
                .zip(weight_matrices.iter())
                .map(|(correspondence_points, weight_matrix)| {
                    (2.0 * correspondence_points.measured.coords.transpose()
                        * rotation_derivative.transpose()
                        * weight_matrix
                        * ((correction * correspondence_points.measured)
                            - correspondence_points.reference))
                        .x
                })
                .sum::<f32>()
                / correspondence_points.len() as f32;
            correction = Isometry2::new(
                correction.translation.vector - gradient_descent_step_size * translation_gradient,
                rotation - gradient_descent_step_size * rotation_gradient,
            );
            if fit_errors_is_subscribed {
                let error = get_fit_error(&correspondence_points, &weight_matrices, correction);
                fit_errors_per_iteration.push(error);
            }
            let gradient_norm = vector![
                translation_gradient.x,
                translation_gradient.y,
                rotation_gradient
            ]
            .norm();
            if gradient_norm < gradient_convergence_threshold {
                break;
            }
        }
        if fit_errors_is_subscribed {
            fit_errors.push(fit_errors_per_iteration);
        }
    }

    let field_mark_correspondences = get_field_mark_correspondence(
        measured_lines_in_field,
        correction,
        field_marks,
        line_length_acceptance_factor,
    );

    let correspondence_points = get_correspondence_points(field_mark_correspondences.clone());
    let weight_matrices: Vec<_> = correspondence_points
        .iter()
        .map(|correspondence_points| {
            let normal =
                (correction * correspondence_points.measured) - correspondence_points.reference;
            if normal.norm() > 0.0 {
                let normal_versor = normal.normalize();
                normal_versor * normal_versor.transpose()
            } else {
                Matrix2::zeros()
            }
        })
        .collect();
    let fit_error = get_fit_error(&correspondence_points, &weight_matrices, correction);

    (field_mark_correspondences, fit_error, fit_errors)
}

fn get_fit_error(
    correspondence_points: &[CorrespondencePoints],
    weight_matrices: &[Matrix2<f32>],
    correction: Isometry2<f32>,
) -> f32 {
    correspondence_points
        .iter()
        .zip(weight_matrices.iter())
        .map(|(correspondence_points, weight_matrix)| {
            ((correction * correspondence_points.measured - correspondence_points.reference)
                .transpose()
                * weight_matrix
                * (correction * correspondence_points.measured - correspondence_points.reference))
                .x
        })
        .sum::<f32>()
        / correspondence_points.len() as f32
}

fn get_field_mark_correspondence(
    measured_lines_in_field: &[Line2],
    correction: Isometry2<f32>,
    field_marks: &[FieldMark],
    line_length_acceptance_factor: f32,
) -> Vec<FieldMarkCorrespondence> {
    measured_lines_in_field
        .iter()
        .filter_map(|&measured_line_in_field| {
            let (correspondences, _weight, field_mark, transformed_line) = field_marks
                .iter()
                .filter_map(|field_mark| {
                    let transformed_line = correction * measured_line_in_field;
                    let field_mark_length = match field_mark {
                        FieldMark::Line { line, direction: _ } => line.length(),
                        FieldMark::Circle { center: _, radius } => *radius, // approximation
                    };
                    let measured_line_length = transformed_line.length();
                    if measured_line_length <= field_mark_length * line_length_acceptance_factor {
                        let correspondences = field_mark.to_correspondence_points(transformed_line);
                        assert_relative_eq!(
                            correspondences.measured_direction.norm(),
                            1.0,
                            epsilon = 0.0001
                        );
                        assert_relative_eq!(
                            correspondences.reference_direction.norm(),
                            1.0,
                            epsilon = 0.0001
                        );
                        let angle_weight = correspondences
                            .measured_direction
                            .dot(&correspondences.reference_direction)
                            .abs()
                            + measured_line_length / field_mark_length;
                        assert!(field_mark_length != 0.0);
                        let length_weight = measured_line_length / field_mark_length; // TODO: this will penalize center circle lines because field_mark_length is only approximated
                        let weight = angle_weight + length_weight;
                        if weight != 0.0 {
                            Some((correspondences, weight, field_mark, transformed_line))
                        } else {
                            None
                        }
                    } else {
                        None
                    }
                })
                .min_by_key(
                    |(correspondence_points, weight, _field_mark, _transformed_line)| {
                        assert!(*weight != 0.0);
                        (NotNan::new(
                            distance(
                                &correspondence_points.correspondence_points.0.measured,
                                &correspondence_points.correspondence_points.0.reference,
                            ) + distance(
                                &correspondence_points.correspondence_points.1.measured,
                                &correspondence_points.correspondence_points.1.reference,
                            ),
                        )
                        .unwrap())
                            / *weight
                    },
                )?;
            let inverse_transformation = correction.inverse();
            Some(FieldMarkCorrespondence {
                measured_line_in_field: inverse_transformation * transformed_line,
                field_mark: *field_mark,
                correspondence_points: (
                    CorrespondencePoints {
                        measured: inverse_transformation
                            * correspondences.correspondence_points.0.measured,
                        reference: correspondences.correspondence_points.0.reference,
                    },
                    CorrespondencePoints {
                        measured: inverse_transformation
                            * correspondences.correspondence_points.1.measured,
                        reference: correspondences.correspondence_points.1.reference,
                    },
                ),
            })
        })
        .collect()
}

fn get_correspondence_points(
    field_mark_correspondences: Vec<FieldMarkCorrespondence>,
) -> Vec<CorrespondencePoints> {
    field_mark_correspondences
        .iter()
        .flat_map(|field_mark_correspondence| {
            [
                field_mark_correspondence.correspondence_points.0,
                field_mark_correspondence.correspondence_points.1,
            ]
        })
        .collect()
}

fn get_translation_and_rotation_measurement(
    robot_to_field: Isometry2<f32>,
    field_mark_correspondence: FieldMarkCorrespondence,
) -> Vector2<f32> {
    let (field_mark_line, field_mark_line_direction) = match field_mark_correspondence.field_mark {
        FieldMark::Line { line, direction } => (line, direction),
        _ => panic!("Expected line mark"),
    };
    let measured_line_in_field = match field_mark_line_direction {
        Direction::PositiveX
            if field_mark_correspondence.measured_line_in_field.1.x
                < field_mark_correspondence.measured_line_in_field.0.x =>
        {
            Line(
                field_mark_correspondence.measured_line_in_field.1,
                field_mark_correspondence.measured_line_in_field.0,
            )
        }
        Direction::PositiveY
            if field_mark_correspondence.measured_line_in_field.1.y
                < field_mark_correspondence.measured_line_in_field.0.y =>
        {
            Line(
                field_mark_correspondence.measured_line_in_field.1,
                field_mark_correspondence.measured_line_in_field.0,
            )
        }
        _ => field_mark_correspondence.measured_line_in_field,
    };
    let measured_line_in_field_vector = measured_line_in_field.1 - measured_line_in_field.0;
    let signed_distance_to_line = measured_line_in_field
        .signed_distance_to_point(Point2::from(robot_to_field.translation.vector));
    match field_mark_line_direction {
        Direction::PositiveX => {
            vector![
                field_mark_line.0.y + signed_distance_to_line,
                (-measured_line_in_field_vector.y).atan2(measured_line_in_field_vector.x)
                    + robot_to_field.rotation.angle()
            ]
        }
        Direction::PositiveY => {
            vector![
                field_mark_line.0.x - signed_distance_to_line,
                measured_line_in_field_vector
                    .x
                    .atan2(measured_line_in_field_vector.y)
                    + robot_to_field.rotation.angle()
            ]
        }
    }
}

fn get_2d_translation_measurement(
    robot_to_field: Isometry2<f32>,
    field_mark_correspondence: FieldMarkCorrespondence,
) -> Vector2<f32> {
    let measured_line_vector = field_mark_correspondence.correspondence_points.1.measured
        - field_mark_correspondence.correspondence_points.0.measured;
    let reference_line_vector = field_mark_correspondence.correspondence_points.1.reference
        - field_mark_correspondence.correspondence_points.0.reference;
    let measured_line_point_0_to_robot_vector = Point2::from(robot_to_field.translation.vector)
        - field_mark_correspondence.correspondence_points.0.measured;
    // Signed angle between two vectors: https://wumbo.net/formula/angle-between-two-vectors-2d/
    let measured_rotation = f32::atan2(
        measured_line_point_0_to_robot_vector.y * measured_line_vector.x
            - measured_line_point_0_to_robot_vector.x * measured_line_vector.y,
        measured_line_point_0_to_robot_vector.x * measured_line_vector.x
            + measured_line_point_0_to_robot_vector.y * measured_line_vector.y,
    );

    let reference_line_point_0_to_robot_vector = Rotation2::new(measured_rotation)
        * reference_line_vector.normalize()
        * measured_line_point_0_to_robot_vector.norm();
    let reference_robot_point = field_mark_correspondence.correspondence_points.0.reference
        + reference_line_point_0_to_robot_vector;
    reference_robot_point.coords
}

pub fn generate_initial_pose(
    initial_pose: &InitialPose,
    field_dimensions: &FieldDimensions,
) -> Isometry2<f32> {
    match initial_pose.side {
        Side::Left => Isometry2::new(
            vector!(
                initial_pose.center_line_offset_x,
                field_dimensions.width * 0.5
            ),
            -FRAC_PI_2,
        ),
        Side::Right => Isometry2::new(
            vector!(
                initial_pose.center_line_offset_x,
                -field_dimensions.width * 0.5
            ),
            FRAC_PI_2,
        ),
    }
}

fn generate_penalized_poses(field_dimensions: &FieldDimensions) -> Vec<Isometry2<f32>> {
    vec![
        Isometry2::new(
            vector!(
                -field_dimensions.length * 0.5 + field_dimensions.penalty_marker_distance,
                -field_dimensions.width * 0.5
            ),
            FRAC_PI_2,
        ),
        Isometry2::new(
            vector!(
                -field_dimensions.length * 0.5 + field_dimensions.penalty_marker_distance,
                field_dimensions.width * 0.5
            ),
            -FRAC_PI_2,
        ),
    ]
}

#[cfg(test)]
mod tests {
    use std::f32::consts::FRAC_PI_4;

    use nalgebra::point;

    use super::*;

    #[test]
    fn signed_angle() {
        let vector0 = vector![1.0_f32, 0.0_f32];
        let vector1 = vector![0.0_f32, 1.0_f32];
        let vector0_angle = vector0.y.atan2(vector0.x);
        let vector1_angle = vector1.y.atan2(vector1.x);
        assert_relative_eq!(vector1_angle - vector0_angle, FRAC_PI_2);
        assert_relative_eq!(vector0_angle - vector1_angle, -FRAC_PI_2);
    }

    #[test]
    fn fitting_line_results_in_zero_measurement() {
        let robot_to_field = Isometry2::identity();
        let field_mark_correspondence = FieldMarkCorrespondence {
            measured_line_in_field: Line(point![0.0, 0.0], point![0.0, 1.0]),
            field_mark: FieldMark::Line {
                line: Line(point![0.0, -3.0], point![0.0, 3.0]),
                direction: Direction::PositiveY,
            },
            correspondence_points: (
                CorrespondencePoints {
                    measured: Point2::origin(),
                    reference: Point2::origin(),
                },
                CorrespondencePoints {
                    measured: Point2::origin(),
                    reference: Point2::origin(),
                },
            ),
        };
        let update =
            get_translation_and_rotation_measurement(robot_to_field, field_mark_correspondence);
        assert_relative_eq!(update, Vector2::zeros());

        let field_mark_correspondence = FieldMarkCorrespondence {
            measured_line_in_field: Line(point![0.0, 1.0], point![0.0, 0.0]),
            field_mark: FieldMark::Line {
                line: Line(point![0.0, -3.0], point![0.0, 3.0]),
                direction: Direction::PositiveY,
            },
            correspondence_points: (
                CorrespondencePoints {
                    measured: Point2::origin(),
                    reference: Point2::origin(),
                },
                CorrespondencePoints {
                    measured: Point2::origin(),
                    reference: Point2::origin(),
                },
            ),
        };
        let update =
            get_translation_and_rotation_measurement(robot_to_field, field_mark_correspondence);
        assert_relative_eq!(update, Vector2::zeros());

        let field_mark_correspondence = FieldMarkCorrespondence {
            measured_line_in_field: Line(point![0.0, 0.0], point![1.0, 0.0]),
            field_mark: FieldMark::Line {
                line: Line(point![-3.0, 0.0], point![3.0, 0.0]),
                direction: Direction::PositiveX,
            },
            correspondence_points: (
                CorrespondencePoints {
                    measured: Point2::origin(),
                    reference: Point2::origin(),
                },
                CorrespondencePoints {
                    measured: Point2::origin(),
                    reference: Point2::origin(),
                },
            ),
        };
        let update =
            get_translation_and_rotation_measurement(robot_to_field, field_mark_correspondence);
        assert_relative_eq!(update, Vector2::zeros());

        let field_mark_correspondence = FieldMarkCorrespondence {
            measured_line_in_field: Line(point![1.0, 0.0], point![0.0, 0.0]),
            field_mark: FieldMark::Line {
                line: Line(point![-3.0, 0.0], point![3.0, 0.0]),
                direction: Direction::PositiveX,
            },
            correspondence_points: (
                CorrespondencePoints {
                    measured: Point2::origin(),
                    reference: Point2::origin(),
                },
                CorrespondencePoints {
                    measured: Point2::origin(),
                    reference: Point2::origin(),
                },
            ),
        };
        let update =
            get_translation_and_rotation_measurement(robot_to_field, field_mark_correspondence);
        assert_relative_eq!(update, Vector2::zeros());
    }

    #[test]
    fn translated_line_results_in_translation_measurement() {
        let robot_to_field = Isometry2::identity();
        let field_mark_correspondence = FieldMarkCorrespondence {
            measured_line_in_field: Line(point![1.0, 0.0], point![1.0, 1.0]),
            field_mark: FieldMark::Line {
                line: Line(point![0.0, -3.0], point![0.0, 3.0]),
                direction: Direction::PositiveY,
            },
            correspondence_points: (
                CorrespondencePoints {
                    measured: Point2::origin(),
                    reference: Point2::origin(),
                },
                CorrespondencePoints {
                    measured: Point2::origin(),
                    reference: Point2::origin(),
                },
            ),
        };
        let update =
            get_translation_and_rotation_measurement(robot_to_field, field_mark_correspondence);
        assert_relative_eq!(update, vector![-1.0, 0.0]);

        let field_mark_correspondence = FieldMarkCorrespondence {
            measured_line_in_field: Line(point![1.0, 1.0], point![1.0, 0.0]),
            field_mark: FieldMark::Line {
                line: Line(point![0.0, -3.0], point![0.0, 3.0]),
                direction: Direction::PositiveY,
            },
            correspondence_points: (
                CorrespondencePoints {
                    measured: Point2::origin(),
                    reference: Point2::origin(),
                },
                CorrespondencePoints {
                    measured: Point2::origin(),
                    reference: Point2::origin(),
                },
            ),
        };
        let update =
            get_translation_and_rotation_measurement(robot_to_field, field_mark_correspondence);
        assert_relative_eq!(update, vector![-1.0, 0.0]);

        let field_mark_correspondence = FieldMarkCorrespondence {
            measured_line_in_field: Line(point![-1.0, 0.0], point![-1.0, 1.0]),
            field_mark: FieldMark::Line {
                line: Line(point![0.0, -3.0], point![0.0, 3.0]),
                direction: Direction::PositiveY,
            },
            correspondence_points: (
                CorrespondencePoints {
                    measured: Point2::origin(),
                    reference: Point2::origin(),
                },
                CorrespondencePoints {
                    measured: Point2::origin(),
                    reference: Point2::origin(),
                },
            ),
        };
        let update =
            get_translation_and_rotation_measurement(robot_to_field, field_mark_correspondence);
        assert_relative_eq!(update, vector![1.0, 0.0]);

        let field_mark_correspondence = FieldMarkCorrespondence {
            measured_line_in_field: Line(point![-1.0, 1.0], point![-1.0, 0.0]),
            field_mark: FieldMark::Line {
                line: Line(point![0.0, -3.0], point![0.0, 3.0]),
                direction: Direction::PositiveY,
            },
            correspondence_points: (
                CorrespondencePoints {
                    measured: Point2::origin(),
                    reference: Point2::origin(),
                },
                CorrespondencePoints {
                    measured: Point2::origin(),
                    reference: Point2::origin(),
                },
            ),
        };
        let update =
            get_translation_and_rotation_measurement(robot_to_field, field_mark_correspondence);
        assert_relative_eq!(update, vector![1.0, 0.0]);

        let field_mark_correspondence = FieldMarkCorrespondence {
            measured_line_in_field: Line(point![0.0, 1.0], point![1.0, 1.0]),
            field_mark: FieldMark::Line {
                line: Line(point![-3.0, 0.0], point![3.0, 0.0]),
                direction: Direction::PositiveX,
            },
            correspondence_points: (
                CorrespondencePoints {
                    measured: Point2::origin(),
                    reference: Point2::origin(),
                },
                CorrespondencePoints {
                    measured: Point2::origin(),
                    reference: Point2::origin(),
                },
            ),
        };
        let update =
            get_translation_and_rotation_measurement(robot_to_field, field_mark_correspondence);
        assert_relative_eq!(update, vector![-1.0, 0.0]);

        let field_mark_correspondence = FieldMarkCorrespondence {
            measured_line_in_field: Line(point![1.0, 1.0], point![0.0, 1.0]),
            field_mark: FieldMark::Line {
                line: Line(point![-3.0, 0.0], point![3.0, 0.0]),
                direction: Direction::PositiveX,
            },
            correspondence_points: (
                CorrespondencePoints {
                    measured: Point2::origin(),
                    reference: Point2::origin(),
                },
                CorrespondencePoints {
                    measured: Point2::origin(),
                    reference: Point2::origin(),
                },
            ),
        };
        let update =
            get_translation_and_rotation_measurement(robot_to_field, field_mark_correspondence);
        assert_relative_eq!(update, vector![-1.0, 0.0]);

        let field_mark_correspondence = FieldMarkCorrespondence {
            measured_line_in_field: Line(point![0.0, -1.0], point![1.0, -1.0]),
            field_mark: FieldMark::Line {
                line: Line(point![-3.0, 0.0], point![3.0, 0.0]),
                direction: Direction::PositiveX,
            },
            correspondence_points: (
                CorrespondencePoints {
                    measured: Point2::origin(),
                    reference: Point2::origin(),
                },
                CorrespondencePoints {
                    measured: Point2::origin(),
                    reference: Point2::origin(),
                },
            ),
        };
        let update =
            get_translation_and_rotation_measurement(robot_to_field, field_mark_correspondence);
        assert_relative_eq!(update, vector![1.0, 0.0]);

        let field_mark_correspondence = FieldMarkCorrespondence {
            measured_line_in_field: Line(point![1.0, -1.0], point![0.0, -1.0]),
            field_mark: FieldMark::Line {
                line: Line(point![-3.0, 0.0], point![3.0, 0.0]),
                direction: Direction::PositiveX,
            },
            correspondence_points: (
                CorrespondencePoints {
                    measured: Point2::origin(),
                    reference: Point2::origin(),
                },
                CorrespondencePoints {
                    measured: Point2::origin(),
                    reference: Point2::origin(),
                },
            ),
        };
        let update =
            get_translation_and_rotation_measurement(robot_to_field, field_mark_correspondence);
        assert_relative_eq!(update, vector![1.0, 0.0]);
    }

    #[test]
    fn rotated_line_results_in_rotation_measurement() {
        let robot_to_field = Isometry2::identity();
        let field_mark_correspondence = FieldMarkCorrespondence {
            measured_line_in_field: Line(point![-1.0, -1.0], point![1.0, 1.0]),
            field_mark: FieldMark::Line {
                line: Line(point![0.0, -3.0], point![0.0, 3.0]),
                direction: Direction::PositiveY,
            },
            correspondence_points: (
                CorrespondencePoints {
                    measured: Point2::origin(),
                    reference: Point2::origin(),
                },
                CorrespondencePoints {
                    measured: Point2::origin(),
                    reference: Point2::origin(),
                },
            ),
        };
        let update =
            get_translation_and_rotation_measurement(robot_to_field, field_mark_correspondence);
        assert_relative_eq!(update, vector![0.0, FRAC_PI_4]);

        let field_mark_correspondence = FieldMarkCorrespondence {
            measured_line_in_field: Line(point![1.0, 1.0], point![-1.0, -1.0]),
            field_mark: FieldMark::Line {
                line: Line(point![0.0, -3.0], point![0.0, 3.0]),
                direction: Direction::PositiveY,
            },
            correspondence_points: (
                CorrespondencePoints {
                    measured: Point2::origin(),
                    reference: Point2::origin(),
                },
                CorrespondencePoints {
                    measured: Point2::origin(),
                    reference: Point2::origin(),
                },
            ),
        };
        let update =
            get_translation_and_rotation_measurement(robot_to_field, field_mark_correspondence);
        assert_relative_eq!(update, vector![0.0, FRAC_PI_4]);

        let field_mark_correspondence = FieldMarkCorrespondence {
            measured_line_in_field: Line(point![-1.0, 1.0], point![1.0, -1.0]),
            field_mark: FieldMark::Line {
                line: Line(point![0.0, -3.0], point![0.0, 3.0]),
                direction: Direction::PositiveY,
            },
            correspondence_points: (
                CorrespondencePoints {
                    measured: Point2::origin(),
                    reference: Point2::origin(),
                },
                CorrespondencePoints {
                    measured: Point2::origin(),
                    reference: Point2::origin(),
                },
            ),
        };
        let update =
            get_translation_and_rotation_measurement(robot_to_field, field_mark_correspondence);
        assert_relative_eq!(update, vector![0.0, -FRAC_PI_4]);

        let field_mark_correspondence = FieldMarkCorrespondence {
            measured_line_in_field: Line(point![1.0, -1.0], point![-1.0, 1.0]),
            field_mark: FieldMark::Line {
                line: Line(point![0.0, -3.0], point![0.0, 3.0]),
                direction: Direction::PositiveY,
            },
            correspondence_points: (
                CorrespondencePoints {
                    measured: Point2::origin(),
                    reference: Point2::origin(),
                },
                CorrespondencePoints {
                    measured: Point2::origin(),
                    reference: Point2::origin(),
                },
            ),
        };
        let update =
            get_translation_and_rotation_measurement(robot_to_field, field_mark_correspondence);
        assert_relative_eq!(update, vector![0.0, -FRAC_PI_4]);

        let field_mark_correspondence = FieldMarkCorrespondence {
            measured_line_in_field: Line(point![-1.0, -1.0], point![1.0, 1.0]),
            field_mark: FieldMark::Line {
                line: Line(point![-3.0, 0.0], point![3.0, 0.0]),
                direction: Direction::PositiveX,
            },
            correspondence_points: (
                CorrespondencePoints {
                    measured: Point2::origin(),
                    reference: Point2::origin(),
                },
                CorrespondencePoints {
                    measured: Point2::origin(),
                    reference: Point2::origin(),
                },
            ),
        };
        let update =
            get_translation_and_rotation_measurement(robot_to_field, field_mark_correspondence);
        assert_relative_eq!(update, vector![0.0, -FRAC_PI_4]);

        let field_mark_correspondence = FieldMarkCorrespondence {
            measured_line_in_field: Line(point![1.0, 1.0], point![-1.0, -1.0]),
            field_mark: FieldMark::Line {
                line: Line(point![-3.0, 0.0], point![3.0, 0.0]),
                direction: Direction::PositiveX,
            },
            correspondence_points: (
                CorrespondencePoints {
                    measured: Point2::origin(),
                    reference: Point2::origin(),
                },
                CorrespondencePoints {
                    measured: Point2::origin(),
                    reference: Point2::origin(),
                },
            ),
        };
        let update =
            get_translation_and_rotation_measurement(robot_to_field, field_mark_correspondence);
        assert_relative_eq!(update, vector![0.0, -FRAC_PI_4]);

        let field_mark_correspondence = FieldMarkCorrespondence {
            measured_line_in_field: Line(point![-1.0, 1.0], point![1.0, -1.0]),
            field_mark: FieldMark::Line {
                line: Line(point![-3.0, 0.0], point![3.0, 0.0]),
                direction: Direction::PositiveX,
            },
            correspondence_points: (
                CorrespondencePoints {
                    measured: Point2::origin(),
                    reference: Point2::origin(),
                },
                CorrespondencePoints {
                    measured: Point2::origin(),
                    reference: Point2::origin(),
                },
            ),
        };
        let update =
            get_translation_and_rotation_measurement(robot_to_field, field_mark_correspondence);
        assert_relative_eq!(update, vector![0.0, FRAC_PI_4]);

        let field_mark_correspondence = FieldMarkCorrespondence {
            measured_line_in_field: Line(point![1.0, -1.0], point![-1.0, 1.0]),
            field_mark: FieldMark::Line {
                line: Line(point![-3.0, 0.0], point![3.0, 0.0]),
                direction: Direction::PositiveX,
            },
            correspondence_points: (
                CorrespondencePoints {
                    measured: Point2::origin(),
                    reference: Point2::origin(),
                },
                CorrespondencePoints {
                    measured: Point2::origin(),
                    reference: Point2::origin(),
                },
            ),
        };
        let update =
            get_translation_and_rotation_measurement(robot_to_field, field_mark_correspondence);
        assert_relative_eq!(update, vector![0.0, FRAC_PI_4]);
    }

    #[test]
    fn correct_correspondence_points() {
        let line_length_acceptance_factor = 1.5;

        let measured_lines_in_field = [Line(point![0.0, 0.0], point![1.0, 0.0])];
        let field_marks = [FieldMark::Line {
            line: Line(point![0.0, 0.0], point![1.0, 0.0]),
            direction: Direction::PositiveX,
        }];
        let correspondences = get_field_mark_correspondence(
            &measured_lines_in_field,
            Isometry2::identity(),
            &field_marks,
            line_length_acceptance_factor,
        );
        assert_eq!(correspondences.len(), 1);
        assert_relative_eq!(
            correspondences[0].correspondence_points.0.measured,
            point![0.0, 0.0]
        );
        assert_relative_eq!(
            correspondences[0].correspondence_points.0.reference,
            point![0.0, 0.0]
        );
        assert_relative_eq!(
            correspondences[0].correspondence_points.1.measured,
            point![1.0, 0.0]
        );
        assert_relative_eq!(
            correspondences[0].correspondence_points.1.reference,
            point![1.0, 0.0]
        );

        let measured_lines_in_field = [Line(point![0.0, 0.0], point![1.0, 0.0])];
        let field_marks = [FieldMark::Line {
            line: Line(point![0.0, 1.0], point![1.0, 1.0]),
            direction: Direction::PositiveX,
        }];
        let correspondences = get_field_mark_correspondence(
            &measured_lines_in_field,
            Isometry2::identity(),
            &field_marks,
            line_length_acceptance_factor,
        );
        assert_eq!(correspondences.len(), 1);
        assert_relative_eq!(
            correspondences[0].correspondence_points.0.measured,
            point![0.0, 0.0]
        );
        assert_relative_eq!(
            correspondences[0].correspondence_points.0.reference,
            point![0.0, 1.0]
        );
        assert_relative_eq!(
            correspondences[0].correspondence_points.1.measured,
            point![1.0, 0.0]
        );
        assert_relative_eq!(
            correspondences[0].correspondence_points.1.reference,
            point![1.0, 1.0]
        );

        let measured_lines_in_field = [Line(point![0.0, 0.0], point![1.0, 0.0])];
        let field_marks = [FieldMark::Line {
            line: Line(point![0.0, 0.0], point![1.0, 0.0]),
            direction: Direction::PositiveX,
        }];
        let correspondences = get_field_mark_correspondence(
            &measured_lines_in_field,
            Isometry2::new(vector![0.0, 1.0], 0.0),
            &field_marks,
            line_length_acceptance_factor,
        );
        assert_eq!(correspondences.len(), 1);
        assert_relative_eq!(
            correspondences[0].correspondence_points.0.measured,
            point![0.0, 0.0]
        );
        assert_relative_eq!(
            correspondences[0].correspondence_points.0.reference,
            point![0.0, 0.0]
        );
        assert_relative_eq!(
            correspondences[0].correspondence_points.1.measured,
            point![1.0, 0.0]
        );
        assert_relative_eq!(
            correspondences[0].correspondence_points.1.reference,
            point![1.0, 0.0]
        );
    }

    #[test]
    fn circle_mark_correspondence_translates() {
        let robot_to_field = Isometry2::identity();
        let field_mark_correspondence = FieldMarkCorrespondence {
            measured_line_in_field: Line(Point2::origin(), Point2::origin()),
            field_mark: FieldMark::Circle {
                center: Point2::origin(),
                radius: 0.0,
            },
            correspondence_points: (
                CorrespondencePoints {
                    measured: point![0.0, 0.0],
                    reference: point![0.0, 0.0],
                },
                CorrespondencePoints {
                    measured: point![1.0, 0.0],
                    reference: point![1.0, 0.0],
                },
            ),
        };
        let update = get_2d_translation_measurement(robot_to_field, field_mark_correspondence);
        assert_relative_eq!(update, vector![0.0, 0.0], epsilon = 0.0001);

        let field_mark_correspondence = FieldMarkCorrespondence {
            measured_line_in_field: Line(Point2::origin(), Point2::origin()),
            field_mark: FieldMark::Circle {
                center: Point2::origin(),
                radius: 0.0,
            },
            correspondence_points: (
                CorrespondencePoints {
                    measured: point![0.0, 1.0],
                    reference: point![0.0, 0.0],
                },
                CorrespondencePoints {
                    measured: point![1.0, 1.0],
                    reference: point![1.0, 0.0],
                },
            ),
        };
        let update = get_2d_translation_measurement(robot_to_field, field_mark_correspondence);
        assert_relative_eq!(update, vector![0.0, -1.0], epsilon = 0.0001);

        let field_mark_correspondence = FieldMarkCorrespondence {
            measured_line_in_field: Line(Point2::origin(), Point2::origin()),
            field_mark: FieldMark::Circle {
                center: Point2::origin(),
                radius: 0.0,
            },
            correspondence_points: (
                CorrespondencePoints {
                    measured: point![0.0, -1.0],
                    reference: point![0.0, 0.0],
                },
                CorrespondencePoints {
                    measured: point![1.0, -1.0],
                    reference: point![1.0, 0.0],
                },
            ),
        };
        let update = get_2d_translation_measurement(robot_to_field, field_mark_correspondence);
        assert_relative_eq!(update, vector![0.0, 1.0], epsilon = 0.0001);

        let field_mark_correspondence = FieldMarkCorrespondence {
            measured_line_in_field: Line(Point2::origin(), Point2::origin()),
            field_mark: FieldMark::Circle {
                center: Point2::origin(),
                radius: 0.0,
            },
            correspondence_points: (
                CorrespondencePoints {
                    measured: point![1.0, 0.0],
                    reference: point![0.0, 0.0],
                },
                CorrespondencePoints {
                    measured: point![1.0, 1.0],
                    reference: point![0.0, 1.0],
                },
            ),
        };
        let update = get_2d_translation_measurement(robot_to_field, field_mark_correspondence);
        assert_relative_eq!(update, vector![-1.0, 0.0], epsilon = 0.0001);

        let field_mark_correspondence = FieldMarkCorrespondence {
            measured_line_in_field: Line(Point2::origin(), Point2::origin()),
            field_mark: FieldMark::Circle {
                center: Point2::origin(),
                radius: 0.0,
            },
            correspondence_points: (
                CorrespondencePoints {
                    measured: point![-1.0, 0.0],
                    reference: point![0.0, 0.0],
                },
                CorrespondencePoints {
                    measured: point![-1.0, 1.0],
                    reference: point![0.0, 1.0],
                },
            ),
        };
        let update = get_2d_translation_measurement(robot_to_field, field_mark_correspondence);
        assert_relative_eq!(update, vector![1.0, 0.0], epsilon = 0.0001);

        let field_mark_correspondence = FieldMarkCorrespondence {
            measured_line_in_field: Line(Point2::origin(), Point2::origin()),
            field_mark: FieldMark::Circle {
                center: Point2::origin(),
                radius: 0.0,
            },
            correspondence_points: (
                CorrespondencePoints {
                    measured: point![1.0, 1.0],
                    reference: point![0.0, 0.0],
                },
                CorrespondencePoints {
                    measured: point![1.0, 2.0],
                    reference: point![0.0, 1.0],
                },
            ),
        };
        let update = get_2d_translation_measurement(robot_to_field, field_mark_correspondence);
        assert_relative_eq!(update, vector![-1.0, -1.0], epsilon = 0.0001);

        let field_mark_correspondence = FieldMarkCorrespondence {
            measured_line_in_field: Line(Point2::origin(), Point2::origin()),
            field_mark: FieldMark::Circle {
                center: Point2::origin(),
                radius: 0.0,
            },
            correspondence_points: (
                CorrespondencePoints {
                    measured: point![1.0, 1.0],
                    reference: point![-1.0, -1.0],
                },
                CorrespondencePoints {
                    measured: point![2.0, 1.0],
                    reference: point![-1.0, 0.0],
                },
            ),
        };
        let update = get_2d_translation_measurement(robot_to_field, field_mark_correspondence);
        assert_relative_eq!(update, vector![0.0, -2.0], epsilon = 0.0001);
    }
}
