use std::{
    f32::consts::{FRAC_PI_2, PI},
    time::{Duration, SystemTime},
};

use approx::assert_relative_eq;
use color_eyre::{eyre::WrapErr, Result};
use geometry::line_segment::LineSegment;
use linear_algebra::{distance, point, IntoTransform, Isometry2, Pose2};
use nalgebra::{matrix, Matrix, Matrix2, Matrix3, Rotation2, Translation2, Vector2, Vector3};
use ordered_float::NotNan;
use serde::{Deserialize, Serialize};

use context_attribute::context;
use coordinate_systems::{Field, Ground};
use filtering::pose_filter::PoseFilter;
use framework::{AdditionalOutput, HistoricInput, MainOutput, PerceptionInput};
use spl_network_messages::{GamePhase, Penalty, SubState, Team};
use types::{
    cycle_time::CycleTime,
    field_dimensions::FieldDimensions,
    field_marks::{field_marks_from_field_dimensions, CorrespondencePoints, Direction, FieldMark},
    filtered_game_controller_state::FilteredGameControllerState,
    initial_pose::InitialPose,
    line_data::LineData,
    localization::{ScoredPose, Update},
    multivariate_normal_distribution::MultivariateNormalDistribution,
    primary_state::PrimaryState,
    support_foot::Side,
};

#[derive(Deserialize, Serialize)]
pub struct Localization {
    field_marks: Vec<FieldMark>,
    last_primary_state: PrimaryState,
    hypotheses: Vec<ScoredPose>,
    hypotheses_when_entered_playing: Vec<ScoredPose>,
    is_penalized_with_motion_in_set_or_initial: bool,
    was_picked_up_while_penalized: bool,
    time_when_penalized_clicked: Option<SystemTime>,
}

#[context]
pub struct CreationContext {
    field_dimensions: Parameter<FieldDimensions, "field_dimensions">,
}

#[context]
pub struct CycleContext {
    correspondence_lines:
        AdditionalOutput<Vec<LineSegment<Field>>, "localization.correspondence_lines">,
    fit_errors: AdditionalOutput<Vec<Vec<Vec<Vec<f32>>>>, "localization.fit_errors">,
    measured_lines_in_field:
        AdditionalOutput<Vec<LineSegment<Field>>, "localization.measured_lines_in_field">,
    pose_hypotheses: AdditionalOutput<Vec<ScoredPose>, "localization.pose_hypotheses">,
    updates: AdditionalOutput<Vec<Vec<Update>>, "localization.updates">,

    current_odometry_to_last_odometry:
        HistoricInput<Option<nalgebra::Isometry2<f32>>, "current_odometry_to_last_odometry?">,

    filtered_game_controller_state:
        Input<Option<FilteredGameControllerState>, "filtered_game_controller_state?">,
    has_ground_contact: Input<bool, "has_ground_contact">,
    primary_state: Input<PrimaryState, "primary_state">,
    walk_in_position_index: Input<usize, "walk_in_position_index">,

    circle_measurement_noise: Parameter<Vector2<f32>, "localization.circle_measurement_noise">,
    field_dimensions: Parameter<FieldDimensions, "field_dimensions">,
    good_matching_threshold: Parameter<f32, "localization.good_matching_threshold">,
    gradient_convergence_threshold: Parameter<f32, "localization.gradient_convergence_threshold">,
    gradient_descent_step_size: Parameter<f32, "localization.gradient_descent_step_size">,
    hypothesis_prediction_score_reduction_factor:
        Parameter<f32, "localization.hypothesis_prediction_score_reduction_factor">,
    hypothesis_retain_factor: Parameter<f32, "localization.hypothesis_retain_factor">,
    hypothesis_score_base_increase: Parameter<f32, "localization.hypothesis_score_base_increase">,
    initial_hypothesis_covariance:
        Parameter<Matrix3<f32>, "localization.initial_hypothesis_covariance">,
    initial_hypothesis_score: Parameter<f32, "localization.initial_hypothesis_score">,
    initial_poses: Parameter<Vec<InitialPose>, "localization.initial_poses">,
    line_length_acceptance_factor: Parameter<f32, "localization.line_length_acceptance_factor">,
    line_measurement_noise: Parameter<Vector2<f32>, "localization.line_measurement_noise">,
    maximum_amount_of_gradient_descent_iterations:
        Parameter<usize, "localization.maximum_amount_of_gradient_descent_iterations">,
    maximum_amount_of_outer_iterations:
        Parameter<usize, "localization.maximum_amount_of_outer_iterations">,
    minimum_fit_error: Parameter<f32, "localization.minimum_fit_error">,
    odometry_noise: Parameter<Vector3<f32>, "localization.odometry_noise">,
    jersey_number: Parameter<usize, "jersey_number">,
    penalized_distance: Parameter<f32, "localization.penalized_distance">,
    penalized_hypothesis_covariance:
        Parameter<Matrix3<f32>, "localization.penalized_hypothesis_covariance">,
    score_per_good_match: Parameter<f32, "localization.score_per_good_match">,
    tentative_penalized_duration: Parameter<Duration, "localization.tentative_penalized_duration">,
    use_line_measurements: Parameter<bool, "localization.use_line_measurements">,
    injected_ground_to_field_of_home_after_coin_toss_before_second_half: Parameter<
        Option<Isometry2<Ground, Field>>,
        "injected_ground_to_field_of_home_after_coin_toss_before_second_half?",
    >,

    line_data_bottom: PerceptionInput<Option<LineData>, "VisionBottom", "line_data?">,
    line_data_top: PerceptionInput<Option<LineData>, "VisionTop", "line_data?">,

    ground_to_field: CyclerState<Isometry2<Ground, Field>, "ground_to_field">,
    cycle_time: Input<CycleTime, "cycle_time">,
}

#[context]
#[derive(Default)]
pub struct MainOutputs {
    pub ground_to_field: MainOutput<Option<Isometry2<Ground, Field>>>,
    pub ground_to_field_of_home_after_coin_toss_before_second_half:
        MainOutput<Option<Isometry2<Ground, Field>>>,
    pub is_localization_converged: MainOutput<bool>,
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
            is_penalized_with_motion_in_set_or_initial: false,
            was_picked_up_while_penalized: false,
            time_when_penalized_clicked: None,
        })
    }

    fn modify_state(
        &mut self,
        context: &CycleContext,
        sub_state: Option<SubState>,
        kicking_team: Option<Team>,
        goalkeeper_jersey_number: Option<usize>,
    ) {
        if let (Some(Team::Opponent), Some(goalkeeper_number), Some(SubState::PenaltyKick)) =
            (kicking_team, goalkeeper_jersey_number, sub_state)
        {
            if goalkeeper_number == *context.jersey_number {
                self.hypotheses = self
                    .hypotheses
                    .iter()
                    .map(|scored_pose| {
                        let mut state = scored_pose.state;
                        state.mean.x = -context.field_dimensions.length / 2.0;

                        ScoredPose {
                            state,
                            score: scored_pose.score,
                        }
                    })
                    .collect();
            }
        }
    }

    fn reset_state(
        &mut self,
        primary_state: PrimaryState,
        game_phase: Option<GamePhase>,
        context: &CycleContext,
        penalty: &Option<Penalty>,
        walk_in_positin_index: usize,
    ) {
        match (self.last_primary_state, primary_state, game_phase) {
            (PrimaryState::Standby | PrimaryState::Initial, PrimaryState::Ready, _) => {
                let initial_pose = generate_initial_pose(
                    &context.initial_poses[walk_in_positin_index],
                    context.field_dimensions,
                );
                self.hypotheses = vec![ScoredPose::from_isometry(
                    initial_pose,
                    *context.initial_hypothesis_covariance,
                    *context.initial_hypothesis_score,
                )];
                self.hypotheses_when_entered_playing
                    .clone_from(&self.hypotheses);
            }
            (
                _,
                PrimaryState::Set,
                Some(GamePhase::PenaltyShootout {
                    kicking_team: Team::Hulks,
                }),
            ) => {
                let penalty_shoot_out_striker_pose = Pose2::from(point![
                    -context.field_dimensions.penalty_area_length
                        + (context.field_dimensions.length / 2.0),
                    0.0,
                ]);
                self.hypotheses = vec![ScoredPose::from_isometry(
                    penalty_shoot_out_striker_pose,
                    *context.initial_hypothesis_covariance,
                    *context.initial_hypothesis_score,
                )];
                self.hypotheses_when_entered_playing
                    .clone_from(&self.hypotheses);
            }
            (
                _,
                PrimaryState::Set | PrimaryState::Playing,
                Some(GamePhase::PenaltyShootout {
                    kicking_team: Team::Opponent,
                }),
            ) => {
                let penalty_shoot_out_keeper_pose =
                    Pose2::from(point![-context.field_dimensions.length / 2.0, 0.0]);
                self.hypotheses = vec![ScoredPose::from_isometry(
                    penalty_shoot_out_keeper_pose,
                    *context.initial_hypothesis_covariance,
                    *context.initial_hypothesis_score,
                )];
                self.hypotheses_when_entered_playing
                    .clone_from(&self.hypotheses);
            }
            (PrimaryState::Set, PrimaryState::Playing, _) => {
                self.hypotheses_when_entered_playing
                    .clone_from(&self.hypotheses);
            }
            (PrimaryState::Ready, PrimaryState::Penalized, _) => {
                self.time_when_penalized_clicked = Some(context.cycle_time.start_time);
                match penalty {
                    Some(Penalty::IllegalMotionInStandby { .. }) => {
                        self.is_penalized_with_motion_in_set_or_initial = true;
                    }
                    Some(_) => {}
                    None => {}
                };
            }
            (PrimaryState::Playing, PrimaryState::Penalized, _) => {
                self.time_when_penalized_clicked = Some(context.cycle_time.start_time);
                match penalty {
                    Some(Penalty::IllegalMotionInSet { .. }) => {
                        self.is_penalized_with_motion_in_set_or_initial = true;
                    }
                    Some(_) => {}
                    None => {}
                };
            }
            (PrimaryState::Penalized, _, _) if primary_state != PrimaryState::Penalized => {
                if self.is_penalized_with_motion_in_set_or_initial {
                    if self.was_picked_up_while_penalized {
                        self.hypotheses
                            .clone_from(&self.hypotheses_when_entered_playing);
                    }
                } else if self.time_when_penalized_clicked.map_or(true, |time| {
                    context
                        .cycle_time
                        .start_time
                        .duration_since(time)
                        .expect("time ran backwards")
                        > *context.tentative_penalized_duration
                }) {
                    let penalized_poses = generate_penalized_poses(
                        context.field_dimensions,
                        *context.penalized_distance,
                    );
                    self.hypotheses = penalized_poses
                        .into_iter()
                        .map(|pose| {
                            ScoredPose::from_isometry(
                                pose,
                                *context.penalized_hypothesis_covariance,
                                *context.initial_hypothesis_score,
                            )
                        })
                        .collect();
                    self.hypotheses_when_entered_playing
                        .clone_from(&self.hypotheses);
                }
                self.is_penalized_with_motion_in_set_or_initial = false;
                self.was_picked_up_while_penalized = false;
            }
            (PrimaryState::Unstiff, _, _) => {
                let penalized_poses =
                    generate_penalized_poses(context.field_dimensions, *context.penalized_distance);
                self.hypotheses = penalized_poses
                    .into_iter()
                    .map(|pose| {
                        ScoredPose::from_isometry(
                            pose,
                            *context.penalized_hypothesis_covariance,
                            *context.initial_hypothesis_score,
                        )
                    })
                    .collect();
                self.hypotheses_when_entered_playing
                    .clone_from(&self.hypotheses);
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

        let line_data = context
            .line_data_top
            .persistent
            .iter()
            .zip(context.line_data_bottom.persistent.iter());
        for (
            (line_data_top_timestamp, line_data_top),
            (line_data_bottom_timestamp, line_data_bottom),
        ) in line_data
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
                    let ground_to_field: Isometry2<Ground, Field> =
                        scored_state.state.as_isometry().framed_transform();
                    let current_measured_lines_in_field: Vec<_> = line_data_top
                        .iter()
                        .chain(line_data_bottom.iter())
                        .filter_map(|data| data.as_ref())
                        .flat_map(|line_data| {
                            line_data.lines.iter().map(|&measured_line_in_ground| {
                                ground_to_field * measured_line_in_ground
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
                                        LineSegment(
                                            correspondence_points_0.measured,
                                            correspondence_points_0.reference,
                                        ),
                                        LineSegment(
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
                                ground_to_field,
                                field_mark_correspondence,
                            ),
                            FieldMark::Circle { .. } => get_2d_translation_measurement(
                                ground_to_field,
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
                        let line_distance_to_robot =
                            distance(line_center_point, ground_to_field.as_pose().position());
                        context.updates.mutate_if_subscribed(|updates| {
                            if let Some(updates) = updates {
                                updates[hypothesis_index].push({
                                    let ground_to_field =
                                        match field_mark_correspondence.field_mark {
                                            FieldMark::Line { line: _, direction } => {
                                                match direction {
                                                    Direction::PositiveX => {
                                                        nalgebra::Isometry2::new(
                                                            nalgebra::vector![
                                                                ground_to_field.translation().x(),
                                                                update.x
                                                            ],
                                                            update.y,
                                                        )
                                                    }
                                                    Direction::PositiveY => {
                                                        nalgebra::Isometry2::new(
                                                            nalgebra::vector![
                                                                update.x,
                                                                ground_to_field.translation().y()
                                                            ],
                                                            update.y,
                                                        )
                                                    }
                                                }
                                            }
                                            FieldMark::Circle { .. } => nalgebra::Isometry2::new(
                                                update,
                                                ground_to_field.orientation().angle(),
                                            ),
                                        }
                                        .framed_transform();
                                    Update {
                                        ground_to_field,
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
                                            nalgebra::vector![state.y, state.z]
                                        }
                                        Direction::PositiveY => {
                                            nalgebra::vector![state.x, state.z]
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
                                    |state| nalgebra::vector![state.x, state.y],
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
        let ground_to_field = best_hypothesis.state.as_isometry();
        self.hypotheses.retain(|scored_state| {
            scored_state.score >= *context.hypothesis_retain_factor * best_score
        });

        context
            .pose_hypotheses
            .fill_if_subscribed(|| self.hypotheses.clone());
        context
            .fit_errors
            .fill_if_subscribed(|| fit_errors_per_measurement);

        *context.ground_to_field = ground_to_field.framed_transform();

        Ok(())
    }

    pub fn cycle(&mut self, mut context: CycleContext) -> Result<MainOutputs> {
        let primary_state = *context.primary_state;
        let penalties = context
            .filtered_game_controller_state
            .map(|game_controller_state| &game_controller_state.penalties);
        let penalty = penalties.and_then(|penalties| penalties[context.jersey_number]);
        let game_phase = context
            .filtered_game_controller_state
            .map(|game_controller_state| game_controller_state.game_phase);
        let sub_state = context
            .filtered_game_controller_state
            .and_then(|game_controller_state| game_controller_state.sub_state);
        let kicking_team = context
            .filtered_game_controller_state
            .map(|game_controller_state| game_controller_state.kicking_team);
        let goalkeeper_jersey_number = context
            .filtered_game_controller_state
            .map(|game_controller_state| game_controller_state.goal_keeper_number);

        self.reset_state(
            primary_state,
            game_phase,
            &context,
            &penalty,
            *context.walk_in_position_index,
        );
        self.modify_state(&context, sub_state, kicking_team, goalkeeper_jersey_number);
        self.last_primary_state = primary_state;

        if primary_state == PrimaryState::Penalized && !context.has_ground_contact {
            self.was_picked_up_while_penalized = true;
        }

        let ground_to_field = match primary_state {
            PrimaryState::Initial | PrimaryState::Standby => Some(
                generate_initial_pose(
                    &context.initial_poses[*context.walk_in_position_index],
                    context.field_dimensions,
                )
                .as_transform(),
            ),
            PrimaryState::Ready | PrimaryState::Set | PrimaryState::Playing => {
                self.update_state(&mut context)?;
                Some(*context.ground_to_field)
            }
            _ => None,
        };
        let ground_to_field_of_home_after_coin_toss_before_second_half = context
            .injected_ground_to_field_of_home_after_coin_toss_before_second_half
            .copied()
            .or_else(|| {
                ground_to_field
                    .and_then(|ground_to_field| {
                        Some((ground_to_field, context.filtered_game_controller_state?))
                    })
                    .map(|(ground_to_field, game_controller_state)| {
                        if !game_controller_state.own_team_is_home_after_coin_toss {
                            (nalgebra::Isometry2::from_parts(
                                Translation2::default(),
                                Rotation2::new(PI).into(),
                            ) * ground_to_field.inner)
                                .framed_transform()
                        } else {
                            ground_to_field
                        }
                    })
            });
        let is_localization_converged = self.hypotheses.len() == 1;

        Ok(MainOutputs {
            ground_to_field: ground_to_field.into(),
            ground_to_field_of_home_after_coin_toss_before_second_half:
                ground_to_field_of_home_after_coin_toss_before_second_half.into(),
            is_localization_converged: is_localization_converged.into(),
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
            line: LineSegment(
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
            line: LineSegment(
                point![
                    -field_dimensions.length / 2.0 - goal_depth,
                    -goal_width / 2.0
                ],
                point![-field_dimensions.length / 2.0, -goal_width / 2.0],
            ),
            direction: Direction::PositiveX,
        },
        FieldMark::Line {
            line: LineSegment(
                point![
                    -field_dimensions.length / 2.0 - goal_depth,
                    goal_width / 2.0
                ],
                point![-field_dimensions.length / 2.0, goal_width / 2.0],
            ),
            direction: Direction::PositiveX,
        },
        FieldMark::Line {
            line: LineSegment(
                point![
                    field_dimensions.length / 2.0 + goal_depth,
                    -goal_width / 2.0
                ],
                point![field_dimensions.length / 2.0 + goal_depth, goal_width / 2.0],
            ),
            direction: Direction::PositiveY,
        },
        FieldMark::Line {
            line: LineSegment(
                point![field_dimensions.length / 2.0, -goal_width / 2.0],
                point![
                    field_dimensions.length / 2.0 + goal_depth,
                    -goal_width / 2.0
                ],
            ),
            direction: Direction::PositiveX,
        },
        FieldMark::Line {
            line: LineSegment(
                point![field_dimensions.length / 2.0, goal_width / 2.0],
                point![field_dimensions.length / 2.0 + goal_depth, goal_width / 2.0],
            ),
            direction: Direction::PositiveX,
        },
    ]
}

#[derive(Clone, Copy, Debug)]
pub struct FieldMarkCorrespondence {
    measured_line_in_field: LineSegment<Field>,
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
    current_odometry_to_last_odometry: &nalgebra::Isometry2<f32>,
    odometry_noise: &Vector3<f32>,
) -> Result<()> {
    let current_orientation_angle = state.mean.z;
    // rotate odometry noise from robot frame to field frame
    let rotated_noise = Rotation2::new(current_orientation_angle) * odometry_noise.xy();
    let process_noise = Matrix::from_diagonal(&nalgebra::vector![
        rotated_noise.x.abs(),
        rotated_noise.y.abs(),
        odometry_noise.z
    ]);

    state.predict(
        |state| {
            // rotate odometry from robot frame to field frame
            let robot_odometry =
                Rotation2::new(state.z) * current_odometry_to_last_odometry.translation.vector;
            nalgebra::vector![
                state.x + robot_odometry.x,
                state.y + robot_odometry.y,
                state.z + current_odometry_to_last_odometry.rotation.angle()
            ]
        },
        process_noise,
    )?;
    Ok(())
}

#[allow(clippy::too_many_arguments)]
pub fn get_fitted_field_mark_correspondence(
    measured_lines_in_field: &[LineSegment<Field>],
    field_marks: &[FieldMark],
    gradient_convergence_threshold: f32,
    gradient_descent_step_size: f32,
    line_length_acceptance_factor: f32,
    maximum_amount_of_gradient_descent_iterations: usize,
    maximum_amount_of_outer_iterations: usize,
    fit_errors_is_subscribed: bool,
) -> (Vec<FieldMarkCorrespondence>, f32, Vec<Vec<f32>>) {
    let mut fit_errors = vec![];
    let mut correction = nalgebra::Isometry2::identity();
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
                let normal = (correction * correspondence_points.measured.inner)
                    - correspondence_points.reference.inner;
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
                        * ((correction * correspondence_points.measured.inner)
                            - correspondence_points.reference.inner)
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
                    (2.0 * correspondence_points.measured.inner.coords.transpose()
                        * rotation_derivative.transpose()
                        * weight_matrix
                        * ((correction * correspondence_points.measured.inner)
                            - correspondence_points.reference.inner))
                        .x
                })
                .sum::<f32>()
                / correspondence_points.len() as f32;
            correction = nalgebra::Isometry2::new(
                correction.translation.vector - (gradient_descent_step_size * translation_gradient),
                rotation - gradient_descent_step_size * rotation_gradient,
            );
            if fit_errors_is_subscribed {
                let error = get_fit_error(&correspondence_points, &weight_matrices, correction);
                fit_errors_per_iteration.push(error);
            }
            let gradient_norm = nalgebra::vector![
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
            let normal = (correction * correspondence_points.measured.inner)
                - correspondence_points.reference.inner;
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
    correction: nalgebra::Isometry2<f32>,
) -> f32 {
    correspondence_points
        .iter()
        .zip(weight_matrices.iter())
        .map(|(correspondence_points, weight_matrix)| {
            ((correction * correspondence_points.measured.inner
                - correspondence_points.reference.inner)
                .transpose()
                * weight_matrix
                * (correction * correspondence_points.measured.inner
                    - correspondence_points.reference.inner))
                .x
        })
        .sum::<f32>()
        / correspondence_points.len() as f32
}

fn get_field_mark_correspondence(
    measured_lines_in_field: &[LineSegment<Field>],
    correction: nalgebra::Isometry2<f32>,
    field_marks: &[FieldMark],
    line_length_acceptance_factor: f32,
) -> Vec<FieldMarkCorrespondence> {
    measured_lines_in_field
        .iter()
        .filter_map(|&measured_line_in_field| {
            let (correspondences, _weight, field_mark, transformed_line) = field_marks
                .iter()
                .filter_map(|field_mark| {
                    let transformed_line = correction.framed_transform() * measured_line_in_field;
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
                            .dot(correspondences.reference_direction)
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
                                correspondence_points.correspondence_points.0.measured,
                                correspondence_points.correspondence_points.0.reference,
                            ) + distance(
                                correspondence_points.correspondence_points.1.measured,
                                correspondence_points.correspondence_points.1.reference,
                            ),
                        )
                        .unwrap())
                            / *weight
                    },
                )?;
            let inverse_transformation = correction.inverse().framed_transform();
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
    ground_to_field: Isometry2<Ground, Field>,
    field_mark_correspondence: FieldMarkCorrespondence,
) -> nalgebra::Vector2<f32> {
    let (field_mark_line, field_mark_line_direction) = match field_mark_correspondence.field_mark {
        FieldMark::Line { line, direction } => (line, direction),
        _ => panic!("Expected line mark"),
    };
    let measured_line_in_field = match field_mark_line_direction {
        Direction::PositiveX
            if field_mark_correspondence.measured_line_in_field.1.x()
                < field_mark_correspondence.measured_line_in_field.0.x() =>
        {
            LineSegment(
                field_mark_correspondence.measured_line_in_field.1,
                field_mark_correspondence.measured_line_in_field.0,
            )
        }
        Direction::PositiveY
            if field_mark_correspondence.measured_line_in_field.1.y()
                < field_mark_correspondence.measured_line_in_field.0.y() =>
        {
            LineSegment(
                field_mark_correspondence.measured_line_in_field.1,
                field_mark_correspondence.measured_line_in_field.0,
            )
        }
        _ => field_mark_correspondence.measured_line_in_field,
    };
    let measured_line_in_field_vector = measured_line_in_field.1 - measured_line_in_field.0;
    let signed_distance_to_line =
        measured_line_in_field.signed_distance_to_point(ground_to_field.as_pose().position());
    match field_mark_line_direction {
        Direction::PositiveX => {
            nalgebra::vector![
                field_mark_line.0.y() + signed_distance_to_line,
                (-measured_line_in_field_vector.y()).atan2(measured_line_in_field_vector.x())
                    + ground_to_field.orientation().angle()
            ]
        }
        Direction::PositiveY => {
            nalgebra::vector![
                field_mark_line.0.x() - signed_distance_to_line,
                measured_line_in_field_vector
                    .x()
                    .atan2(measured_line_in_field_vector.y())
                    + ground_to_field.orientation().angle()
            ]
        }
    }
}

fn get_2d_translation_measurement(
    ground_to_field: Isometry2<Ground, Field>,
    field_mark_correspondence: FieldMarkCorrespondence,
) -> Vector2<f32> {
    let measured_line_vector = field_mark_correspondence.correspondence_points.1.measured
        - field_mark_correspondence.correspondence_points.0.measured;
    let reference_line_vector = field_mark_correspondence.correspondence_points.1.reference
        - field_mark_correspondence.correspondence_points.0.reference;
    let measured_line_point_0_to_robot_vector = ground_to_field.as_pose().position()
        - field_mark_correspondence.correspondence_points.0.measured;
    // Signed angle between two vectors: https://wumbo.net/formula/angle-between-two-vectors-2d/
    let measured_rotation = f32::atan2(
        measured_line_point_0_to_robot_vector.y() * measured_line_vector.x()
            - measured_line_point_0_to_robot_vector.x() * measured_line_vector.y(),
        measured_line_point_0_to_robot_vector.x() * measured_line_vector.x()
            + measured_line_point_0_to_robot_vector.y() * measured_line_vector.y(),
    );

    let reference_line_point_0_to_robot_vector = Rotation2::new(measured_rotation)
        * reference_line_vector.normalize().inner
        * measured_line_point_0_to_robot_vector.norm();
    let reference_robot_point = field_mark_correspondence
        .correspondence_points
        .0
        .reference
        .inner
        + reference_line_point_0_to_robot_vector;
    reference_robot_point.coords
}

pub fn generate_initial_pose(
    initial_pose: &InitialPose,
    field_dimensions: &FieldDimensions,
) -> Pose2<Field> {
    match initial_pose.side {
        Side::Left => Pose2::new(
            point![
                initial_pose.center_line_offset_x,
                field_dimensions.width * 0.5
            ],
            -FRAC_PI_2,
        ),
        Side::Right => Pose2::new(
            point![
                initial_pose.center_line_offset_x,
                -field_dimensions.width * 0.5
            ],
            FRAC_PI_2,
        ),
    }
}

fn generate_penalized_poses(
    field_dimensions: &FieldDimensions,
    penalized_distance: f32,
) -> Vec<Pose2<Field>> {
    vec![
        Pose2::new(
            point![
                -field_dimensions.length * 0.5 + field_dimensions.penalty_marker_distance,
                -field_dimensions.width * 0.5 - penalized_distance
            ],
            FRAC_PI_2,
        ),
        Pose2::new(
            point![
                -field_dimensions.length * 0.5 + field_dimensions.penalty_marker_distance,
                field_dimensions.width * 0.5 + penalized_distance
            ],
            -FRAC_PI_2,
        ),
    ]
}

#[cfg(test)]
mod tests {
    use std::f32::consts::FRAC_PI_4;

    use linear_algebra::Point2;

    use super::*;

    #[test]
    fn signed_angle() {
        let vector0 = nalgebra::vector![1.0_f32, 0.0_f32];
        let vector1 = nalgebra::vector![0.0_f32, 1.0_f32];
        let vector0_angle = vector0.y.atan2(vector0.x);
        let vector1_angle = vector1.y.atan2(vector1.x);
        assert_relative_eq!(vector1_angle - vector0_angle, FRAC_PI_2);
        assert_relative_eq!(vector0_angle - vector1_angle, -FRAC_PI_2);
    }

    #[test]
    fn fitting_line_results_in_zero_measurement() {
        let ground_to_field = Isometry2::identity();
        let field_mark_correspondence = FieldMarkCorrespondence {
            measured_line_in_field: LineSegment(point![0.0, 0.0], point![0.0, 1.0]),
            field_mark: FieldMark::Line {
                line: LineSegment(point![0.0, -3.0], point![0.0, 3.0]),
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
            get_translation_and_rotation_measurement(ground_to_field, field_mark_correspondence);
        assert_relative_eq!(update, Vector2::zeros());

        let field_mark_correspondence = FieldMarkCorrespondence {
            measured_line_in_field: LineSegment(point![0.0, 1.0], point![0.0, 0.0]),
            field_mark: FieldMark::Line {
                line: LineSegment(point![0.0, -3.0], point![0.0, 3.0]),
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
            get_translation_and_rotation_measurement(ground_to_field, field_mark_correspondence);
        assert_relative_eq!(update, Vector2::zeros());

        let field_mark_correspondence = FieldMarkCorrespondence {
            measured_line_in_field: LineSegment(point![0.0, 0.0], point![1.0, 0.0]),
            field_mark: FieldMark::Line {
                line: LineSegment(point![-3.0, 0.0], point![3.0, 0.0]),
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
            get_translation_and_rotation_measurement(ground_to_field, field_mark_correspondence);
        assert_relative_eq!(update, Vector2::zeros());

        let field_mark_correspondence = FieldMarkCorrespondence {
            measured_line_in_field: LineSegment(point![1.0, 0.0], point![0.0, 0.0]),
            field_mark: FieldMark::Line {
                line: LineSegment(point![-3.0, 0.0], point![3.0, 0.0]),
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
            get_translation_and_rotation_measurement(ground_to_field, field_mark_correspondence);
        assert_relative_eq!(update, Vector2::zeros());
    }

    #[test]
    fn translated_line_results_in_translation_measurement() {
        let ground_to_field = Isometry2::identity();
        let field_mark_correspondence = FieldMarkCorrespondence {
            measured_line_in_field: LineSegment(point![1.0, 0.0], point![1.0, 1.0]),
            field_mark: FieldMark::Line {
                line: LineSegment(point![0.0, -3.0], point![0.0, 3.0]),
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
            get_translation_and_rotation_measurement(ground_to_field, field_mark_correspondence);
        assert_relative_eq!(update, nalgebra::vector![-1.0, 0.0]);

        let field_mark_correspondence = FieldMarkCorrespondence {
            measured_line_in_field: LineSegment(point![1.0, 1.0], point![1.0, 0.0]),
            field_mark: FieldMark::Line {
                line: LineSegment(point![0.0, -3.0], point![0.0, 3.0]),
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
            get_translation_and_rotation_measurement(ground_to_field, field_mark_correspondence);
        assert_relative_eq!(update, nalgebra::vector![-1.0, 0.0]);

        let field_mark_correspondence = FieldMarkCorrespondence {
            measured_line_in_field: LineSegment(point![-1.0, 0.0], point![-1.0, 1.0]),
            field_mark: FieldMark::Line {
                line: LineSegment(point![0.0, -3.0], point![0.0, 3.0]),
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
            get_translation_and_rotation_measurement(ground_to_field, field_mark_correspondence);
        assert_relative_eq!(update, nalgebra::vector![1.0, 0.0]);

        let field_mark_correspondence = FieldMarkCorrespondence {
            measured_line_in_field: LineSegment(point![-1.0, 1.0], point![-1.0, 0.0]),
            field_mark: FieldMark::Line {
                line: LineSegment(point![0.0, -3.0], point![0.0, 3.0]),
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
            get_translation_and_rotation_measurement(ground_to_field, field_mark_correspondence);
        assert_relative_eq!(update, nalgebra::vector![1.0, 0.0]);

        let field_mark_correspondence = FieldMarkCorrespondence {
            measured_line_in_field: LineSegment(point![0.0, 1.0], point![1.0, 1.0]),
            field_mark: FieldMark::Line {
                line: LineSegment(point![-3.0, 0.0], point![3.0, 0.0]),
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
            get_translation_and_rotation_measurement(ground_to_field, field_mark_correspondence);
        assert_relative_eq!(update, nalgebra::vector![-1.0, 0.0]);

        let field_mark_correspondence = FieldMarkCorrespondence {
            measured_line_in_field: LineSegment(point![1.0, 1.0], point![0.0, 1.0]),
            field_mark: FieldMark::Line {
                line: LineSegment(point![-3.0, 0.0], point![3.0, 0.0]),
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
            get_translation_and_rotation_measurement(ground_to_field, field_mark_correspondence);
        assert_relative_eq!(update, nalgebra::vector![-1.0, 0.0]);

        let field_mark_correspondence = FieldMarkCorrespondence {
            measured_line_in_field: LineSegment(point![0.0, -1.0], point![1.0, -1.0]),
            field_mark: FieldMark::Line {
                line: LineSegment(point![-3.0, 0.0], point![3.0, 0.0]),
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
            get_translation_and_rotation_measurement(ground_to_field, field_mark_correspondence);
        assert_relative_eq!(update, nalgebra::vector![1.0, 0.0]);

        let field_mark_correspondence = FieldMarkCorrespondence {
            measured_line_in_field: LineSegment(point![1.0, -1.0], point![0.0, -1.0]),
            field_mark: FieldMark::Line {
                line: LineSegment(point![-3.0, 0.0], point![3.0, 0.0]),
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
            get_translation_and_rotation_measurement(ground_to_field, field_mark_correspondence);
        assert_relative_eq!(update, nalgebra::vector![1.0, 0.0]);
    }

    #[test]
    fn rotated_line_results_in_rotation_measurement() {
        let ground_to_field = Isometry2::identity();
        let field_mark_correspondence = FieldMarkCorrespondence {
            measured_line_in_field: LineSegment(point![-1.0, -1.0], point![1.0, 1.0]),
            field_mark: FieldMark::Line {
                line: LineSegment(point![0.0, -3.0], point![0.0, 3.0]),
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
            get_translation_and_rotation_measurement(ground_to_field, field_mark_correspondence);
        assert_relative_eq!(update, nalgebra::vector![0.0, FRAC_PI_4]);

        let field_mark_correspondence = FieldMarkCorrespondence {
            measured_line_in_field: LineSegment(point![1.0, 1.0], point![-1.0, -1.0]),
            field_mark: FieldMark::Line {
                line: LineSegment(point![0.0, -3.0], point![0.0, 3.0]),
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
            get_translation_and_rotation_measurement(ground_to_field, field_mark_correspondence);
        assert_relative_eq!(update, nalgebra::vector![0.0, FRAC_PI_4]);

        let field_mark_correspondence = FieldMarkCorrespondence {
            measured_line_in_field: LineSegment(point![-1.0, 1.0], point![1.0, -1.0]),
            field_mark: FieldMark::Line {
                line: LineSegment(point![0.0, -3.0], point![0.0, 3.0]),
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
            get_translation_and_rotation_measurement(ground_to_field, field_mark_correspondence);
        assert_relative_eq!(update, nalgebra::vector![0.0, -FRAC_PI_4]);

        let field_mark_correspondence = FieldMarkCorrespondence {
            measured_line_in_field: LineSegment(point![1.0, -1.0], point![-1.0, 1.0]),
            field_mark: FieldMark::Line {
                line: LineSegment(point![0.0, -3.0], point![0.0, 3.0]),
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
            get_translation_and_rotation_measurement(ground_to_field, field_mark_correspondence);
        assert_relative_eq!(update, nalgebra::vector![0.0, -FRAC_PI_4]);

        let field_mark_correspondence = FieldMarkCorrespondence {
            measured_line_in_field: LineSegment(point![-1.0, -1.0], point![1.0, 1.0]),
            field_mark: FieldMark::Line {
                line: LineSegment(point![-3.0, 0.0], point![3.0, 0.0]),
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
            get_translation_and_rotation_measurement(ground_to_field, field_mark_correspondence);
        assert_relative_eq!(update, nalgebra::vector![0.0, -FRAC_PI_4]);

        let field_mark_correspondence = FieldMarkCorrespondence {
            measured_line_in_field: LineSegment(point![1.0, 1.0], point![-1.0, -1.0]),
            field_mark: FieldMark::Line {
                line: LineSegment(point![-3.0, 0.0], point![3.0, 0.0]),
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
            get_translation_and_rotation_measurement(ground_to_field, field_mark_correspondence);
        assert_relative_eq!(update, nalgebra::vector![0.0, -FRAC_PI_4]);

        let field_mark_correspondence = FieldMarkCorrespondence {
            measured_line_in_field: LineSegment(point![-1.0, 1.0], point![1.0, -1.0]),
            field_mark: FieldMark::Line {
                line: LineSegment(point![-3.0, 0.0], point![3.0, 0.0]),
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
            get_translation_and_rotation_measurement(ground_to_field, field_mark_correspondence);
        assert_relative_eq!(update, nalgebra::vector![0.0, FRAC_PI_4]);

        let field_mark_correspondence = FieldMarkCorrespondence {
            measured_line_in_field: LineSegment(point![1.0, -1.0], point![-1.0, 1.0]),
            field_mark: FieldMark::Line {
                line: LineSegment(point![-3.0, 0.0], point![3.0, 0.0]),
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
            get_translation_and_rotation_measurement(ground_to_field, field_mark_correspondence);
        assert_relative_eq!(update, nalgebra::vector![0.0, FRAC_PI_4]);
    }

    #[test]
    fn correct_correspondence_points() {
        let line_length_acceptance_factor = 1.5;

        let measured_lines_in_field = [LineSegment(point![0.0, 0.0], point![1.0, 0.0])];
        let field_marks = [FieldMark::Line {
            line: LineSegment(point![0.0, 0.0], point![1.0, 0.0]),
            direction: Direction::PositiveX,
        }];
        let correspondences = get_field_mark_correspondence(
            &measured_lines_in_field,
            nalgebra::Isometry2::identity(),
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

        let measured_lines_in_field = [LineSegment(point![0.0, 0.0], point![1.0, 0.0])];
        let field_marks = [FieldMark::Line {
            line: LineSegment(point![0.0, 1.0], point![1.0, 1.0]),
            direction: Direction::PositiveX,
        }];
        let correspondences = get_field_mark_correspondence(
            &measured_lines_in_field,
            nalgebra::Isometry2::identity(),
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

        let measured_lines_in_field = [LineSegment(point![0.0, 0.0], point![1.0, 0.0])];
        let field_marks = [FieldMark::Line {
            line: LineSegment(point![0.0, 0.0], point![1.0, 0.0]),
            direction: Direction::PositiveX,
        }];
        let correspondences = get_field_mark_correspondence(
            &measured_lines_in_field,
            nalgebra::Isometry2::new(nalgebra::vector![0.0, 1.0], 0.0),
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
        let ground_to_field = Isometry2::identity();
        let field_mark_correspondence = FieldMarkCorrespondence {
            measured_line_in_field: LineSegment(Point2::origin(), Point2::origin()),
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
        let update = get_2d_translation_measurement(ground_to_field, field_mark_correspondence);
        assert_relative_eq!(update, nalgebra::vector![0.0, 0.0], epsilon = 0.0001);

        let field_mark_correspondence = FieldMarkCorrespondence {
            measured_line_in_field: LineSegment(Point2::origin(), Point2::origin()),
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
        let update = get_2d_translation_measurement(ground_to_field, field_mark_correspondence);
        assert_relative_eq!(update, nalgebra::vector![0.0, -1.0], epsilon = 0.0001);

        let field_mark_correspondence = FieldMarkCorrespondence {
            measured_line_in_field: LineSegment(Point2::origin(), Point2::origin()),
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
        let update = get_2d_translation_measurement(ground_to_field, field_mark_correspondence);
        assert_relative_eq!(update, nalgebra::vector![0.0, 1.0], epsilon = 0.0001);

        let field_mark_correspondence = FieldMarkCorrespondence {
            measured_line_in_field: LineSegment(Point2::origin(), Point2::origin()),
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
        let update = get_2d_translation_measurement(ground_to_field, field_mark_correspondence);
        assert_relative_eq!(update, nalgebra::vector![-1.0, 0.0], epsilon = 0.0001);

        let field_mark_correspondence = FieldMarkCorrespondence {
            measured_line_in_field: LineSegment(Point2::origin(), Point2::origin()),
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
        let update = get_2d_translation_measurement(ground_to_field, field_mark_correspondence);
        assert_relative_eq!(update, nalgebra::vector![1.0, 0.0], epsilon = 0.0001);

        let field_mark_correspondence = FieldMarkCorrespondence {
            measured_line_in_field: LineSegment(Point2::origin(), Point2::origin()),
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
        let update = get_2d_translation_measurement(ground_to_field, field_mark_correspondence);
        assert_relative_eq!(update, nalgebra::vector![-1.0, -1.0], epsilon = 0.0001);

        let field_mark_correspondence = FieldMarkCorrespondence {
            measured_line_in_field: LineSegment(Point2::origin(), Point2::origin()),
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
        let update = get_2d_translation_measurement(ground_to_field, field_mark_correspondence);
        assert_relative_eq!(update, nalgebra::vector![0.0, -2.0], epsilon = 0.0001);
    }
}
