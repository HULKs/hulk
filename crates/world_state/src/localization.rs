use std::{
    f32::consts::FRAC_PI_2,
    time::{Duration, SystemTime},
};

use booster::{FallDownState, FallDownStateType, ImuState, Odometer};
use color_eyre::{
    Result,
    eyre::{Context, OptionExt},
};
use geometry::line_segment::LineSegment;
use linear_algebra::{IntoTransform, Isometry2, Point2, Pose2, distance, point};
use nalgebra::{Matrix2, Matrix3, Rotation2, Vector2, Vector3, matrix};
use ordered_float::NotNan;
use serde::{Deserialize, Serialize};

use context_attribute::context;
use coordinate_systems::{Field, Ground};
use filtering::pose_filter::PoseFilter;
use framework::{AdditionalOutput, MainOutput, PerceptionInput};
use hsl_network_messages::{GamePhase, Penalty, PlayerNumber, SubState, Team};
use types::{
    cycle_time::CycleTime,
    field_dimensions::FieldDimensions,
    field_marks::{CorrespondencePoints, Direction, FieldMark, field_marks_from_field_dimensions},
    filtered_game_controller_state::FilteredGameControllerState,
    initial_pose::InitialPose,
    line_data::LineData,
    localization::{ScoredPose, Update},
    multivariate_normal_distribution::MultivariateNormalDistribution,
    players::Players,
    primary_state::PrimaryState,
    support_foot::Side,
};

type FitErrorsPerGradientStep = Vec<f32>;
type FitErrorsPerOuterIteration = Vec<FitErrorsPerGradientStep>;
type FitErrorsPerHypothesis = Vec<FitErrorsPerOuterIteration>;
type FitErrorsPerMeasurement = Vec<FitErrorsPerHypothesis>;

#[derive(Deserialize, Serialize)]
pub struct Localization {
    field_marks: Vec<FieldMark>,
    last_primary_state: PrimaryState,
    hypotheses: Vec<ScoredPose>,
    hypotheses_when_entered_playing: Vec<ScoredPose>,
    is_penalized_with_motion_in_set_or_initial: bool,
    time_when_penalized_clicked: Option<SystemTime>,
    last_odometer: Option<Odometer>,
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
    gyro_movement: AdditionalOutput<f32, "localization.gyro_movement">,

    filtered_game_controller_state:
        Input<Option<FilteredGameControllerState>, "filtered_game_controller_state?">,
    primary_state: Input<PrimaryState, "primary_state">,
    cycle_time: Input<CycleTime, "cycle_time">,

    odometer: PerceptionInput<Odometer, "Odometry", "odometer">,
    fall_down_state: PerceptionInput<Option<FallDownState>, "FallDownState", "fall_down_state?">,
    imu_state: PerceptionInput<ImuState, "Motion", "imu_state">,

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
    initial_poses: Parameter<Players<InitialPose>, "localization.initial_poses">,
    line_length_acceptance_factor: Parameter<f32, "localization.line_length_acceptance_factor">,
    line_measurement_noise: Parameter<Vector2<f32>, "localization.line_measurement_noise">,
    additional_moving_noise_line:
        Parameter<Vector2<f32>, "localization.additional_moving_noise_line">,
    additional_moving_noise_circle:
        Parameter<Vector2<f32>, "localization.additional_moving_noise_circle">,
    maximum_amount_of_gradient_descent_iterations:
        Parameter<usize, "localization.maximum_amount_of_gradient_descent_iterations">,
    maximum_amount_of_outer_iterations:
        Parameter<usize, "localization.maximum_amount_of_outer_iterations">,
    minimum_fit_error: Parameter<f32, "localization.minimum_fit_error">,
    odometry_noise: Parameter<Vector3<f32>, "localization.odometry_noise">,
    player_number: Parameter<PlayerNumber, "player_number">,
    penalized_distance: Parameter<f32, "localization.penalized_distance">,
    penalized_hypothesis_covariance:
        Parameter<Matrix3<f32>, "localization.penalized_hypothesis_covariance">,
    score_per_good_match: Parameter<f32, "localization.score_per_good_match">,
    tentative_penalized_duration: Parameter<Duration, "localization.tentative_penalized_duration">,
    use_line_measurements: Parameter<bool, "localization.use_line_measurements">,
}

#[context]
#[derive(Default)]
pub struct MainOutputs {
    pub ground_to_field: MainOutput<Option<Isometry2<Ground, Field>>>,
    pub is_localization_converged: MainOutput<bool>,
}

#[derive(Default)]
struct MockLineMeasurementSource {
    batch: LineData,
}

impl MockLineMeasurementSource {
    fn batch(&self) -> &LineData {
        &self.batch
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum PenaltyExitStrategy {
    KeepCurrent,
    RestorePlaying,
    ResetToPenalized,
}

struct MeasurementNoise {
    line: Matrix2<f32>,
    circle: Matrix2<f32>,
}

struct DebugUpdateContext {
    hypothesis_index: usize,
    field_mark_correspondence: FieldMarkCorrespondence,
    ground_to_field: Isometry2<Ground, Field>,
    update: Vector2<f32>,
    clamped_fit_error: f32,
    number_of_measurements_weight: f32,
    line_length_weight: f32,
    line_center_point: Point2<Field>,
    line_distance_to_robot: f32,
}

struct CycleInputs {
    cycle_start_time: SystemTime,
    primary_state: PrimaryState,
    game_phase: Option<GamePhase>,
    sub_state: Option<SubState>,
    kicking_team: Option<Team>,
    penalty: Option<Penalty>,
    gyro_movement: f32,
    line_measurements_allowed: bool,
    current_odometry_to_last_odometry: Option<nalgebra::Isometry2<f32>>,
    measurement_source: MockLineMeasurementSource,
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
            last_primary_state: PrimaryState::Safe,
            hypotheses: Vec::new(),
            hypotheses_when_entered_playing: Vec::new(),
            is_penalized_with_motion_in_set_or_initial: false,
            time_when_penalized_clicked: None,
            last_odometer: None,
        })
    }

    pub fn cycle(&mut self, mut context: CycleContext) -> Result<MainOutputs> {
        let inputs = self.capture_cycle_inputs(&context);

        self.handle_state_transition(&inputs, &context);
        self.ensure_hypotheses_seeded(inputs.primary_state, &context);
        self.apply_sub_state_adjustments(&context, inputs.sub_state, inputs.kicking_team);
        self.last_primary_state = inputs.primary_state;

        let ground_to_field = self.pose_for_cycle(&inputs, &mut context)?;

        Ok(self.compose_main_outputs(&inputs, &mut context, ground_to_field))
    }

    fn capture_cycle_inputs(&mut self, context: &CycleContext) -> CycleInputs {
        let cycle_start_time = context.cycle_time.start_time;

        let primary_state = *context.primary_state;

        let game_phase = context
            .filtered_game_controller_state
            .map(|game_controller_state| game_controller_state.game_phase);

        let sub_state = context
            .filtered_game_controller_state
            .and_then(|game_controller_state| game_controller_state.sub_state);

        let kicking_team = context
            .filtered_game_controller_state
            .and_then(|game_controller_state| game_controller_state.kicking_team);

        let penalty = context
            .filtered_game_controller_state
            .and_then(|game_controller_state| {
                game_controller_state.penalties[*context.player_number]
            });

        let gyro_movement = Self::latest_imu_state(context).angular_velocity.norm();

        let fall_down_state = Self::latest_fall_down_state(context);
        let line_measurements_allowed = !matches!(
            fall_down_state,
            Some(
                FallDownStateType::IsFalling
                    | FallDownStateType::HasFallen
                    | FallDownStateType::IsGettingUp
            )
        );

        let newest_odometer = Self::latest_odometer(context);
        let current_odometry_to_last_odometry = newest_odometer.as_ref().and_then(|odometer| {
            self.last_odometer
                .as_ref()
                .map(|last_odometer| odometry_delta(last_odometer, odometer))
        });
        self.last_odometer = newest_odometer;

        let measurement_source = MockLineMeasurementSource::default();

        CycleInputs {
            cycle_start_time,
            primary_state,
            game_phase,
            sub_state,
            kicking_team,
            penalty,
            gyro_movement,
            line_measurements_allowed,
            current_odometry_to_last_odometry,
            measurement_source,
        }
    }

    fn latest_odometer(context: &CycleContext) -> Option<Odometer> {
        context
            .odometer
            .persistent
            .iter()
            .chain(context.odometer.temporary.iter())
            .flat_map(|(_timestamp, odometers)| odometers.iter().cloned().cloned())
            .next_back()
    }

    fn latest_fall_down_state(context: &CycleContext) -> Option<FallDownStateType> {
        context
            .fall_down_state
            .persistent
            .iter()
            .chain(context.fall_down_state.temporary.iter())
            .flat_map(|(_timestamp, states)| states.iter())
            .filter_map(|state| state.as_ref().map(|state| state.fall_down_state))
            .next_back()
    }

    fn latest_imu_state(context: &CycleContext) -> ImuState {
        context
            .imu_state
            .persistent
            .iter()
            .chain(context.imu_state.temporary.iter())
            .flat_map(|(_timestamp, imu_states)| imu_states.iter().copied().copied())
            .next_back()
            .unwrap_or_default()
    }

    fn handle_state_transition(&mut self, inputs: &CycleInputs, context: &CycleContext) {
        match (
            self.last_primary_state,
            inputs.primary_state,
            inputs.game_phase,
        ) {
            (last_state, PrimaryState::Initial, _)
                if last_state != PrimaryState::Initial && last_state != PrimaryState::Penalized =>
            {
                self.seed_from_initial_pose(context);
            }
            (
                _,
                PrimaryState::Set,
                Some(GamePhase::PenaltyShootout {
                    kicking_team: Team::Hulks,
                }),
            ) => self.seed_from_single_pose(
                Pose2::from(point![
                    -context.field_dimensions.penalty_area_length
                        + (context.field_dimensions.length / 2.0),
                    0.0,
                ]),
                context,
            ),
            (
                _,
                PrimaryState::Set | PrimaryState::Playing,
                Some(GamePhase::PenaltyShootout {
                    kicking_team: Team::Opponent,
                }),
            ) => self.seed_from_single_pose(
                Pose2::from(point![-context.field_dimensions.length / 2.0, 0.0]),
                context,
            ),
            (PrimaryState::Set, PrimaryState::Playing, _) => {
                self.hypotheses_when_entered_playing
                    .clone_from(&self.hypotheses);
            }
            (
                PrimaryState::Playing | PrimaryState::Ready | PrimaryState::Set,
                PrimaryState::Penalized,
                _,
            ) => {
                self.time_when_penalized_clicked = Some(inputs.cycle_start_time);
                self.is_penalized_with_motion_in_set_or_initial =
                    matches!(inputs.penalty, Some(Penalty::MotionInSet { .. }));
            }
            (PrimaryState::Penalized, _, _) if inputs.primary_state != PrimaryState::Penalized => {
                match penalty_exit_strategy(
                    self.is_penalized_with_motion_in_set_or_initial,
                    self.time_when_penalized_clicked,
                    inputs.cycle_start_time,
                    *context.tentative_penalized_duration,
                ) {
                    PenaltyExitStrategy::KeepCurrent => {}
                    PenaltyExitStrategy::RestorePlaying => {
                        self.hypotheses
                            .clone_from(&self.hypotheses_when_entered_playing);
                    }
                    PenaltyExitStrategy::ResetToPenalized => {
                        self.seed_penalized_hypotheses(context);
                    }
                }
                self.is_penalized_with_motion_in_set_or_initial = false;
            }
            _ => {}
        }
    }

    fn apply_sub_state_adjustments(
        &mut self,
        context: &CycleContext,
        sub_state: Option<SubState>,
        kicking_team: Option<Team>,
    ) {
        if let (PlayerNumber::One, Some(SubState::PenaltyKick)) =
            (*context.player_number, sub_state)
        {
            if matches!(kicking_team, Some(Team::Opponent)) {
                for hypothesis in &mut self.hypotheses {
                    hypothesis.state.mean.x = -context.field_dimensions.length / 2.0;
                }
            }
        }
    }

    fn seed_hypotheses(&mut self, hypotheses: Vec<ScoredPose>) {
        self.hypotheses = hypotheses;
        self.hypotheses_when_entered_playing
            .clone_from(&self.hypotheses);
    }

    fn seed_from_single_pose(&mut self, pose: Pose2<Field>, context: &CycleContext) {
        self.seed_hypotheses(vec![ScoredPose::from_isometry(
            pose,
            *context.initial_hypothesis_covariance,
            *context.initial_hypothesis_score,
        )]);
    }

    fn seed_from_initial_pose(&mut self, context: &CycleContext) {
        self.seed_from_single_pose(
            generate_initial_pose(
                &context.initial_poses[*context.player_number],
                context.field_dimensions,
            ),
            context,
        );
    }

    fn seed_penalized_hypotheses(&mut self, context: &CycleContext) {
        self.seed_hypotheses(
            generate_penalized_poses(context.field_dimensions, *context.penalized_distance)
                .into_iter()
                .map(|pose| {
                    ScoredPose::from_isometry(
                        pose,
                        *context.penalized_hypothesis_covariance,
                        *context.initial_hypothesis_score,
                    )
                })
                .collect(),
        );
    }

    fn ensure_hypotheses_seeded(&mut self, primary_state: PrimaryState, context: &CycleContext) {
        if self.hypotheses.is_empty() && primary_state_uses_localization(primary_state) {
            self.seed_from_initial_pose(context);
        }
    }

    fn pose_for_cycle(
        &mut self,
        inputs: &CycleInputs,
        context: &mut CycleContext,
    ) -> Result<Option<Isometry2<Ground, Field>>> {
        Ok(match inputs.primary_state {
            PrimaryState::Initial => Some(
                generate_initial_pose(
                    &context.initial_poses[*context.player_number],
                    context.field_dimensions,
                )
                .as_transform(),
            ),
            PrimaryState::Ready | PrimaryState::Set | PrimaryState::Playing => {
                Some(self.update_active_state(inputs, context)?)
            }
            PrimaryState::Safe
            | PrimaryState::Stop
            | PrimaryState::Penalized
            | PrimaryState::Finished => None,
        })
    }

    fn update_active_state(
        &mut self,
        inputs: &CycleInputs,
        context: &mut CycleContext,
    ) -> Result<Isometry2<Ground, Field>> {
        self.prepare_debug_outputs(context, inputs);
        let measurement_noise = self.measurement_noise(context, inputs);
        self.predict_hypotheses(context, inputs.current_odometry_to_last_odometry.as_ref())?;
        let fit_errors_per_measurement =
            self.apply_measurements(inputs, context, &measurement_noise)?;
        self.finalize_hypotheses(context, fit_errors_per_measurement)
    }

    fn prepare_debug_outputs(&self, context: &mut CycleContext, inputs: &CycleInputs) {
        context.measured_lines_in_field.fill_if_subscribed(Vec::new);
        context.correspondence_lines.fill_if_subscribed(Vec::new);
        context
            .updates
            .fill_if_subscribed(|| vec![vec![]; self.hypotheses.len()]);
        context
            .gyro_movement
            .fill_if_subscribed(|| inputs.gyro_movement);
    }

    fn measurement_noise(&self, context: &CycleContext, inputs: &CycleInputs) -> MeasurementNoise {
        MeasurementNoise {
            line: Matrix2::from_diagonal(
                &(context.line_measurement_noise
                    + context.additional_moving_noise_line * inputs.gyro_movement),
            ),
            circle: Matrix2::from_diagonal(
                &(context.circle_measurement_noise
                    + context.additional_moving_noise_circle * inputs.gyro_movement),
            ),
        }
    }

    fn predict_hypotheses(
        &mut self,
        context: &CycleContext,
        current_odometry_to_last_odometry: Option<&nalgebra::Isometry2<f32>>,
    ) -> Result<()> {
        if let Some(current_odometry_to_last_odometry) = current_odometry_to_last_odometry {
            for scored_state in &mut self.hypotheses {
                predict(
                    &mut scored_state.state,
                    current_odometry_to_last_odometry,
                    context.odometry_noise,
                )
                .wrap_err("failed to predict pose filter")?;
                scored_state.score *= *context.hypothesis_prediction_score_reduction_factor;
            }
        }
        Ok(())
    }

    fn apply_measurements(
        &mut self,
        inputs: &CycleInputs,
        context: &mut CycleContext,
        measurement_noise: &MeasurementNoise,
    ) -> Result<FitErrorsPerMeasurement> {
        if !*context.use_line_measurements || !inputs.line_measurements_allowed {
            return Ok(Vec::new());
        }

        let fit_errors = self.apply_measurement_batch(
            context,
            inputs.measurement_source.batch(),
            measurement_noise,
        )?;
        Ok((!fit_errors.is_empty())
            .then_some(fit_errors)
            .into_iter()
            .collect())
    }

    fn apply_measurement_batch(
        &mut self,
        context: &mut CycleContext,
        line_data: &LineData,
        measurement_noise: &MeasurementNoise,
    ) -> Result<FitErrorsPerHypothesis> {
        let mut fit_errors_per_hypothesis = Vec::with_capacity(self.hypotheses.len());

        for (hypothesis_index, scored_state) in self.hypotheses.iter_mut().enumerate() {
            let ground_to_field: Isometry2<Ground, Field> =
                scored_state.state.as_isometry().framed_transform();
            let measured_lines_in_field: Vec<_> = line_data
                .lines
                .iter()
                .map(|&measured_line_in_ground| ground_to_field * measured_line_in_ground)
                .collect();
            Self::append_measured_lines_for_debug(context, &measured_lines_in_field);

            if measured_lines_in_field.is_empty() {
                continue;
            }

            let (field_mark_correspondences, fit_error, fit_errors) =
                get_fitted_field_mark_correspondence(
                    &measured_lines_in_field,
                    &self.field_marks,
                    *context.gradient_convergence_threshold,
                    *context.gradient_descent_step_size,
                    *context.line_length_acceptance_factor,
                    *context.maximum_amount_of_gradient_descent_iterations,
                    *context.maximum_amount_of_outer_iterations,
                    context.fit_errors.is_subscribed(),
                );

            Self::append_correspondence_lines_for_debug(context, &field_mark_correspondences);

            if field_mark_correspondences.is_empty() {
                continue;
            }

            if context.fit_errors.is_subscribed() {
                fit_errors_per_hypothesis.push(fit_errors);
            }

            let clamped_fit_error = fit_error.max(*context.minimum_fit_error);
            let number_of_measurements_weight = 1.0 / field_mark_correspondences.len() as f32;

            for field_mark_correspondence in field_mark_correspondences {
                let update = match field_mark_correspondence.field_mark {
                    FieldMark::Line { .. } => get_translation_and_rotation_measurement(
                        ground_to_field,
                        field_mark_correspondence,
                    ),
                    FieldMark::Circle { .. } => {
                        get_2d_translation_measurement(ground_to_field, field_mark_correspondence)
                    }
                };
                let line_length = field_mark_correspondence.measured_line_in_field.length();
                let line_length_weight = if line_length == 0.0 {
                    1.0
                } else {
                    1.0 / line_length
                };
                let line_center_point = field_mark_correspondence.measured_line_in_field.center();
                let line_distance_to_robot =
                    distance(line_center_point, ground_to_field.as_pose().position());

                Self::append_update_for_debug(
                    context,
                    DebugUpdateContext {
                        hypothesis_index,
                        field_mark_correspondence,
                        ground_to_field,
                        update,
                        clamped_fit_error,
                        number_of_measurements_weight,
                        line_length_weight,
                        line_center_point,
                        line_distance_to_robot,
                    },
                );

                let uncertainty_weight = clamped_fit_error
                    * number_of_measurements_weight
                    * line_length_weight
                    * line_distance_to_robot;

                match field_mark_correspondence.field_mark {
                    FieldMark::Line { direction, .. } => scored_state
                        .state
                        .update_with_1d_translation_and_rotation(
                            update,
                            measurement_noise.line * uncertainty_weight,
                            |state| match direction {
                                Direction::PositiveX => nalgebra::vector![state.y, state.z],
                                Direction::PositiveY => nalgebra::vector![state.x, state.z],
                            },
                        )
                        .context("failed to update pose filter with line correspondence")?,
                    FieldMark::Circle { .. } => scored_state
                        .state
                        .update_with_2d_translation(
                            update,
                            measurement_noise.circle * uncertainty_weight,
                            |state| nalgebra::vector![state.x, state.y],
                        )
                        .context("failed to update pose filter with circle correspondence")?,
                }

                if field_mark_correspondence.fit_error_sum() < *context.good_matching_threshold {
                    scored_state.score += *context.score_per_good_match;
                }
            }

            scored_state.score += *context.hypothesis_score_base_increase;
        }

        Ok(fit_errors_per_hypothesis)
    }

    fn append_measured_lines_for_debug(
        context: &mut CycleContext,
        measured_lines_in_field: &[LineSegment<Field>],
    ) {
        context
            .measured_lines_in_field
            .mutate_if_subscribed(|existing_lines| {
                if let Some(existing_lines) = existing_lines {
                    existing_lines.extend(measured_lines_in_field.iter());
                }
            });
    }

    fn append_correspondence_lines_for_debug(
        context: &mut CycleContext,
        field_mark_correspondences: &[FieldMarkCorrespondence],
    ) {
        context
            .correspondence_lines
            .mutate_if_subscribed(|correspondence_lines| {
                if let Some(correspondence_lines) = correspondence_lines {
                    correspondence_lines.extend(field_mark_correspondences.iter().flat_map(
                        |field_mark_correspondence| {
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
                        },
                    ));
                }
            });
    }

    fn append_update_for_debug(context: &mut CycleContext, debug: DebugUpdateContext) {
        context.updates.mutate_if_subscribed(|updates| {
            if let Some(updates) = updates {
                let debug_ground_to_field = match debug.field_mark_correspondence.field_mark {
                    FieldMark::Line { direction, .. } => match direction {
                        Direction::PositiveX => nalgebra::Isometry2::new(
                            nalgebra::vector![
                                debug.ground_to_field.translation().x(),
                                debug.update.x
                            ],
                            debug.update.y,
                        ),
                        Direction::PositiveY => nalgebra::Isometry2::new(
                            nalgebra::vector![
                                debug.update.x,
                                debug.ground_to_field.translation().y()
                            ],
                            debug.update.y,
                        ),
                    },
                    FieldMark::Circle { .. } => nalgebra::Isometry2::new(
                        debug.update,
                        debug.ground_to_field.orientation().angle(),
                    ),
                }
                .framed_transform();
                updates[debug.hypothesis_index].push(Update {
                    ground_to_field: debug_ground_to_field,
                    line_center_point: debug.line_center_point,
                    fit_error: debug.clamped_fit_error,
                    number_of_measurements_weight: debug.number_of_measurements_weight,
                    line_distance_to_robot: debug.line_distance_to_robot,
                    line_length_weight: debug.line_length_weight,
                });
            }
        });
    }

    fn finalize_hypotheses(
        &mut self,
        context: &mut CycleContext,
        fit_errors_per_measurement: FitErrorsPerMeasurement,
    ) -> Result<Isometry2<Ground, Field>> {
        let best_hypothesis = self
            .get_best_hypothesis()
            .ok_or_eyre("localization has no pose hypotheses after update")?;
        let best_score = best_hypothesis.score;
        let ground_to_field = best_hypothesis.state.as_isometry();
        self.hypotheses.retain(|scored_state| {
            scored_state.score >= *context.hypothesis_retain_factor * best_score
        });

        context
            .fit_errors
            .fill_if_subscribed(|| fit_errors_per_measurement);

        Ok(ground_to_field.framed_transform())
    }

    fn compose_main_outputs(
        &self,
        _inputs: &CycleInputs,
        context: &mut CycleContext,
        ground_to_field: Option<Isometry2<Ground, Field>>,
    ) -> MainOutputs {
        let is_localization_converged = self.hypotheses.len() == 1;

        context
            .pose_hypotheses
            .fill_if_subscribed(|| self.hypotheses.clone());

        MainOutputs {
            ground_to_field: ground_to_field.into(),
            is_localization_converged: is_localization_converged.into(),
        }
    }

    fn get_best_hypothesis(&self) -> Option<&ScoredPose> {
        self.hypotheses
            .iter()
            .max_by_key(|scored_filter| NotNan::new(scored_filter.score).unwrap())
    }
}

fn primary_state_uses_localization(primary_state: PrimaryState) -> bool {
    matches!(
        primary_state,
        PrimaryState::Initial | PrimaryState::Ready | PrimaryState::Set | PrimaryState::Playing
    )
}

fn penalty_exit_strategy(
    is_penalized_with_motion_in_set_or_initial: bool,
    time_when_penalized_clicked: Option<SystemTime>,
    cycle_start_time: SystemTime,
    tentative_penalized_duration: Duration,
) -> PenaltyExitStrategy {
    if is_penalized_with_motion_in_set_or_initial {
        return PenaltyExitStrategy::RestorePlaying;
    }

    if time_when_penalized_clicked.is_none_or(|time| {
        cycle_start_time
            .duration_since(time)
            .map_or(true, |duration| duration > tentative_penalized_duration)
    }) {
        PenaltyExitStrategy::ResetToPenalized
    } else {
        PenaltyExitStrategy::KeepCurrent
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
    let rotated_noise = Rotation2::new(current_orientation_angle) * odometry_noise.xy();
    let process_noise = Matrix3::from_diagonal(&nalgebra::vector![
        rotated_noise.x.abs(),
        rotated_noise.y.abs(),
        odometry_noise.z
    ]);

    state.predict(
        |state| {
            let last_ground_to_field =
                nalgebra::Isometry2::new(nalgebra::vector![state.x, state.y], state.z);
            let current_ground_to_field = last_ground_to_field * current_odometry_to_last_odometry;

            nalgebra::vector![
                current_ground_to_field.translation.vector.x,
                current_ground_to_field.translation.vector.y,
                current_ground_to_field.rotation.angle()
            ]
        },
        process_noise,
    )?;
    Ok(())
}

fn odometry_delta(last_odometer: &Odometer, odometer: &Odometer) -> nalgebra::Isometry2<f32> {
    let last_odometry_to_world = nalgebra::Isometry2::new(
        nalgebra::vector![last_odometer.x, last_odometer.y],
        last_odometer.theta,
    );
    let current_odometry_to_world =
        nalgebra::Isometry2::new(nalgebra::vector![odometer.x, odometer.y], odometer.theta);

    last_odometry_to_world.inverse() * current_odometry_to_world
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
) -> (
    Vec<FieldMarkCorrespondence>,
    f32,
    FitErrorsPerOuterIteration,
) {
    if measured_lines_in_field.is_empty() || field_marks.is_empty() {
        return (Vec::new(), f32::INFINITY, Vec::new());
    }

    let mut fit_errors = Vec::new();
    let mut correction = nalgebra::Isometry2::identity();

    for _ in 0..maximum_amount_of_outer_iterations {
        let correspondence_points = get_correspondence_points(get_field_mark_correspondence(
            measured_lines_in_field,
            correction,
            field_marks,
            line_length_acceptance_factor,
        ));
        if correspondence_points.is_empty() {
            return (Vec::new(), f32::INFINITY, fit_errors);
        }

        let weight_matrices = weight_matrices(&correspondence_points, correction);
        let mut fit_errors_per_iteration = Vec::new();

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
                fit_errors_per_iteration.push(get_fit_error(
                    &correspondence_points,
                    &weight_matrices,
                    correction,
                ));
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
    if correspondence_points.is_empty() {
        return (Vec::new(), f32::INFINITY, fit_errors);
    }

    let fit_error = get_fit_error(
        &correspondence_points,
        &weight_matrices(&correspondence_points, correction),
        correction,
    );

    (field_mark_correspondences, fit_error, fit_errors)
}

fn weight_matrices(
    correspondence_points: &[CorrespondencePoints],
    correction: nalgebra::Isometry2<f32>,
) -> Vec<Matrix2<f32>> {
    correspondence_points
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
        .collect()
}

fn get_fit_error(
    correspondence_points: &[CorrespondencePoints],
    weight_matrices: &[Matrix2<f32>],
    correction: nalgebra::Isometry2<f32>,
) -> f32 {
    if correspondence_points.is_empty() {
        return f32::INFINITY;
    }

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
                        FieldMark::Line { line, .. } => line.length(),
                        FieldMark::Circle { radius, .. } => *radius,
                    };
                    if field_mark_length <= 0.0 {
                        return None;
                    }

                    let measured_line_length = transformed_line.length();
                    if measured_line_length > field_mark_length * line_length_acceptance_factor {
                        return None;
                    }

                    let correspondences = field_mark.to_correspondence_points(transformed_line);
                    let angle_weight = correspondences
                        .measured_direction
                        .dot(&correspondences.reference_direction)
                        .abs()
                        + measured_line_length / field_mark_length;
                    let length_weight = measured_line_length / field_mark_length;
                    let weight = angle_weight + length_weight;

                    (weight > 0.0).then_some((
                        correspondences,
                        weight,
                        field_mark,
                        transformed_line,
                    ))
                })
                .min_by_key(
                    |(correspondence_points, weight, _field_mark, _transformed_line)| {
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
) -> Vector2<f32> {
    let (field_mark_line, field_mark_line_direction) = match field_mark_correspondence.field_mark {
        FieldMark::Line { line, direction } => (line, direction),
        FieldMark::Circle { .. } => unreachable!("line measurement requested for circle mark"),
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
        Direction::PositiveX => nalgebra::vector![
            field_mark_line.0.y() + signed_distance_to_line,
            (-measured_line_in_field_vector.y()).atan2(measured_line_in_field_vector.x())
                + ground_to_field.orientation().angle()
        ],
        Direction::PositiveY => nalgebra::vector![
            field_mark_line.0.x() - signed_distance_to_line,
            measured_line_in_field_vector
                .x()
                .atan2(measured_line_in_field_vector.y())
                + ground_to_field.orientation().angle()
        ],
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
    use std::{
        f32::consts::{FRAC_PI_4, PI},
        time::{Duration, SystemTime},
    };

    use approx::assert_relative_eq;
    use linear_algebra::Point2;

    use super::*;

    #[test]
    fn penalty_exit_strategy_restores_playing_hypotheses_for_motion_in_set() {
        assert_eq!(
            penalty_exit_strategy(
                true,
                Some(SystemTime::UNIX_EPOCH),
                SystemTime::UNIX_EPOCH + Duration::from_secs(1),
                Duration::from_secs(10),
            ),
            PenaltyExitStrategy::RestorePlaying
        );
    }

    #[test]
    fn penalty_exit_strategy_resets_to_penalized_hypotheses_after_timeout() {
        assert_eq!(
            penalty_exit_strategy(
                false,
                Some(SystemTime::UNIX_EPOCH),
                SystemTime::UNIX_EPOCH + Duration::from_secs(11),
                Duration::from_secs(10),
            ),
            PenaltyExitStrategy::ResetToPenalized
        );
    }

    #[test]
    fn penalty_exit_strategy_keeps_current_hypotheses_before_timeout() {
        assert_eq!(
            penalty_exit_strategy(
                false,
                Some(SystemTime::UNIX_EPOCH),
                SystemTime::UNIX_EPOCH + Duration::from_secs(9),
                Duration::from_secs(10),
            ),
            PenaltyExitStrategy::KeepCurrent
        );
    }

    #[test]
    fn penalty_exit_strategy_resets_when_penalty_time_is_missing() {
        assert_eq!(
            penalty_exit_strategy(
                false,
                None,
                SystemTime::UNIX_EPOCH + Duration::from_secs(9),
                Duration::from_secs(10),
            ),
            PenaltyExitStrategy::ResetToPenalized
        );
    }

    #[test]
    fn empty_measurements_produce_no_correspondence() {
        let (correspondences, fit_error, fit_errors) = get_fitted_field_mark_correspondence(
            &[],
            &[FieldMark::Line {
                line: LineSegment(point![0.0, 0.0], point![1.0, 0.0]),
                direction: Direction::PositiveX,
            }],
            0.001,
            0.1,
            1.5,
            8,
            4,
            true,
        );

        assert!(correspondences.is_empty());
        assert!(fit_error.is_infinite());
        assert!(fit_errors.is_empty());
    }

    #[test]
    fn zero_length_field_marks_are_ignored() {
        let correspondences = get_field_mark_correspondence(
            &[LineSegment(point![0.0, 0.0], point![1.0, 0.0])],
            nalgebra::Isometry2::identity(),
            &[FieldMark::Circle {
                center: Point2::origin(),
                radius: 0.0,
            }],
            1.5,
        );

        assert!(correspondences.is_empty());
    }

    #[test]
    fn odometry_delta_uses_relative_motion() {
        let last_odometer = Odometer {
            x: 1.0,
            y: 2.0,
            theta: FRAC_PI_2,
        };
        let odometer = Odometer {
            x: 1.0,
            y: 3.0,
            theta: FRAC_PI_2 + 0.2,
        };
        let delta = odometry_delta(&last_odometer, &odometer);

        assert_relative_eq!(delta.translation.vector.x, 1.0, epsilon = 0.0001);
        assert_relative_eq!(delta.translation.vector.y, 0.0, epsilon = 0.0001);
        assert_relative_eq!(delta.rotation.angle(), 0.2, epsilon = 0.0001);
    }

    #[test]
    fn odometry_delta_normalizes_rotation_difference() {
        let last_odometer = Odometer {
            x: 0.0,
            y: 0.0,
            theta: 0.1,
        };
        let odometer = Odometer {
            x: 0.0,
            y: 0.0,
            theta: 0.1 + PI + 0.2,
        };
        let delta = odometry_delta(&last_odometer, &odometer);

        assert_relative_eq!(delta.rotation.angle(), -PI + 0.2, epsilon = 0.0001);
    }

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
    }
}
