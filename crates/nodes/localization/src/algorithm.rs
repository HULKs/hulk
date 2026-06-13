use std::{
    collections::BTreeMap,
    f32::consts::FRAC_PI_2,
    time::{Duration, SystemTime},
};

use booster::{FallDownState, FallDownStateType, ImuState, Odometer};
use color_eyre::{
    Result,
    eyre::{Context, OptionExt},
};
use coordinate_systems::{Field, Ground};
use filtering::pose_filter::PoseFilter;
use geometry::line_segment::LineSegment;
use hsl_network_messages::{GamePhase, Penalty, PlayerNumber, SubState, Team};
use linear_algebra::{IntoTransform, Isometry2, Point2, Pose2, distance, point};
use nalgebra::{Matrix2, Matrix3, Rotation2, Vector2, Vector3, matrix};
use serde::{Deserialize, Serialize};
use types::{
    field_dimensions::FieldDimensions,
    field_marks::{CorrespondencePoints, Direction, FieldMark, field_marks_from_field_dimensions},
    filtered_game_controller_state::FilteredGameControllerState,
    initial_pose::InitialPose,
    line_data::LineData,
    localization::{ScoredPose, Update},
    multivariate_normal_distribution::MultivariateNormalDistribution,
    primary_state::PrimaryState,
    support_foot::Side,
};

use crate::Parameters;

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
    #[serde(default)]
    last_imu_state: Option<ImuState>,
    #[serde(default)]
    last_fall_down_state: Option<FallDownStateType>,
    last_line_data_time: SystemTime,
}

pub struct CreationContext<'a> {
    pub field_dimensions: &'a FieldDimensions,
}

#[derive(Debug, Default)]
pub struct PerceptionInput<T> {
    pub persistent: BTreeMap<SystemTime, Vec<T>>,
    pub temporary: BTreeMap<SystemTime, Vec<T>>,
}

pub struct CycleContext<'a> {
    pub correspondence_lines: DebugOutput<Vec<LineSegment<Field>>>,
    pub fit_errors: DebugOutput<Vec<Vec<Vec<Vec<f32>>>>>,
    pub measured_lines_in_field: DebugOutput<Vec<LineSegment<Field>>>,
    pub pose_hypotheses: DebugOutput<Vec<ScoredPose>>,
    pub updates: DebugOutput<Vec<Vec<Update>>>,
    pub gyro_movement: DebugOutput<f32>,

    pub filtered_game_controller_state: Option<&'a FilteredGameControllerState>,
    pub primary_state: &'a PrimaryState,
    pub cycle_start_time: SystemTime,

    pub odometer: PerceptionInput<Odometer>,
    pub fall_down_state: PerceptionInput<FallDownState>,
    pub imu_state: PerceptionInput<ImuState>,
    pub line_data: PerceptionInput<LineData>,

    pub parameters: &'a Parameters,
    pub field_dimensions: &'a FieldDimensions,
    pub player_number: &'a PlayerNumber,
}

pub struct CycleOutputs {
    pub ground_to_field: Option<Isometry2<Ground, Field>>,
    pub is_localization_converged: bool,
    pub correspondence_lines: Option<Vec<LineSegment<Field>>>,
    pub fit_errors: Option<Vec<Vec<Vec<Vec<f32>>>>>,
    pub measured_lines_in_field: Option<Vec<LineSegment<Field>>>,
    pub pose_hypotheses: Option<Vec<ScoredPose>>,
    pub updates: Option<Vec<Vec<Update>>>,
    pub gyro_movement: Option<f32>,
}

#[derive(Debug)]
pub struct DebugOutput<T> {
    subscribed: bool,
    value: Option<T>,
}

impl<T> DebugOutput<T> {
    pub fn new(subscribed: bool) -> Self {
        Self {
            subscribed,
            value: None,
        }
    }

    pub fn is_subscribed(&self) -> bool {
        self.subscribed
    }

    pub fn fill_if_subscribed(&mut self, build: impl FnOnce() -> T) {
        if self.subscribed {
            self.value = Some(build());
        }
    }

    pub fn mutate_if_subscribed(&mut self, mutate: impl FnOnce(Option<&mut T>)) {
        if self.subscribed {
            mutate(self.value.as_mut());
        }
    }

    pub fn take(self) -> Option<T> {
        self.value
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
    current_odometry_to_last_odometry: nalgebra::Isometry2<f32>,
    line_data: Option<LineData>,
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
            last_imu_state: None,
            last_fall_down_state: None,
            last_line_data_time: SystemTime::UNIX_EPOCH,
        })
    }

    pub fn cycle(&mut self, mut context: CycleContext<'_>) -> Result<CycleOutputs> {
        let inputs = self.capture_cycle_inputs(&context);

        self.handle_state_transition(&inputs, &context);
        self.ensure_hypotheses_seeded(inputs.primary_state, &context);
        self.apply_sub_state_adjustments(&context, inputs.sub_state, inputs.kicking_team);
        self.last_primary_state = inputs.primary_state;

        let ground_to_field = self.pose_for_cycle(&inputs, &mut context)?;
        let is_localization_converged = ground_to_field.is_some() && self.hypotheses.len() == 1;

        context
            .pose_hypotheses
            .fill_if_subscribed(|| self.hypotheses.clone());

        Ok(CycleOutputs {
            ground_to_field,
            is_localization_converged,
            correspondence_lines: context.correspondence_lines.take(),
            fit_errors: context.fit_errors.take(),
            measured_lines_in_field: context.measured_lines_in_field.take(),
            pose_hypotheses: context.pose_hypotheses.take(),
            updates: context.updates.take(),
            gyro_movement: context.gyro_movement.take(),
        })
    }

    fn capture_cycle_inputs(&mut self, context: &CycleContext) -> CycleInputs {
        let cycle_start_time = context.cycle_start_time;

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

        if let Some(imu_state) = Self::latest_imu_state(context) {
            self.last_imu_state = Some(imu_state);
        }
        let imu_state = self.last_imu_state.unwrap_or_default();
        let gyro_movement = imu_state.angular_velocity.norm();

        if let Some(fall_down_state) = Self::latest_fall_down_state(context) {
            self.last_fall_down_state = Some(fall_down_state);
        }
        let fall_down_state = self.last_fall_down_state;
        let line_measurements_allowed = !matches!(
            fall_down_state,
            Some(
                FallDownStateType::IsFalling
                    | FallDownStateType::HasFallen
                    | FallDownStateType::IsGettingUp
            )
        );

        let current_odometer = Self::latest_odometer(context);
        let odometer_with_imu_yaw = current_odometer.map(|Odometer { x, y, theta: _ }| Odometer {
            x,
            y,
            theta: imu_state.roll_pitch_yaw.z(),
        });
        let current_odometry_to_last_odometry = match (self.last_odometer, odometer_with_imu_yaw) {
            (Some(last), Some(latest)) => odometry_delta(last, latest),
            _ => Default::default(),
        };
        if let Some(odometer_with_imu_yaw) = odometer_with_imu_yaw {
            self.last_odometer = Some(odometer_with_imu_yaw);
        }

        let line_data = context
            .line_data
            .persistent
            .iter()
            .chain(&context.line_data.temporary)
            .filter(|(time, _)| **time > self.last_line_data_time)
            .flat_map(|(time, detections)| detections.last().map(|data| (*time, data.clone())))
            .last();

        let line_data = match line_data {
            Some((time, data)) => {
                self.last_line_data_time = time;
                Some(data)
            }
            _ => None,
        };

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
            line_data,
        }
    }

    fn latest_odometer(context: &CycleContext) -> Option<Odometer> {
        context
            .odometer
            .persistent
            .iter()
            .chain(context.odometer.temporary.iter())
            .flat_map(|(_timestamp, odometers)| odometers.iter().copied())
            .next_back()
    }

    fn latest_fall_down_state(context: &CycleContext) -> Option<FallDownStateType> {
        context
            .fall_down_state
            .persistent
            .iter()
            .chain(context.fall_down_state.temporary.iter())
            .flat_map(|(_timestamp, states)| states.iter())
            .map(|state| state.fall_down_state)
            .next_back()
    }

    fn latest_imu_state(context: &CycleContext) -> Option<ImuState> {
        context
            .imu_state
            .persistent
            .iter()
            .chain(context.imu_state.temporary.iter())
            .flat_map(|(_timestamp, imu_states)| imu_states.iter().copied())
            .next_back()
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
                    context.parameters.tentative_penalized_duration,
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
            && matches!(kicking_team, Some(Team::Opponent))
        {
            for hypothesis in &mut self.hypotheses {
                hypothesis.state.mean.x = -context.field_dimensions.length / 2.0;
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
            context.parameters.initial_hypothesis_covariance,
            context.parameters.initial_hypothesis_score,
        )]);
    }

    fn seed_from_initial_pose(&mut self, context: &CycleContext) {
        self.seed_from_single_pose(
            generate_initial_pose(
                &context.parameters.initial_poses[*context.player_number],
                context.field_dimensions,
            ),
            context,
        );
    }

    fn seed_penalized_hypotheses(&mut self, context: &CycleContext) {
        self.seed_hypotheses(
            generate_penalized_poses(
                context.field_dimensions,
                context.parameters.penalized_distance,
            )
            .into_iter()
            .map(|pose| {
                ScoredPose::from_isometry(
                    pose,
                    context.parameters.penalized_hypothesis_covariance,
                    context.parameters.initial_hypothesis_score,
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
                    &context.parameters.initial_poses[*context.player_number],
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
        self.predict_hypotheses(context, inputs.current_odometry_to_last_odometry)?;
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
                &(context.parameters.line_measurement_noise
                    + context.parameters.additional_moving_noise_line * inputs.gyro_movement),
            ),
            circle: Matrix2::from_diagonal(
                &(context.parameters.circle_measurement_noise
                    + context.parameters.additional_moving_noise_circle * inputs.gyro_movement),
            ),
        }
    }

    fn predict_hypotheses(
        &mut self,
        context: &CycleContext,
        current_odometry_to_last_odometry: nalgebra::Isometry2<f32>,
    ) -> Result<()> {
        for scored_state in &mut self.hypotheses {
            predict(
                &mut scored_state.state,
                current_odometry_to_last_odometry,
                &context.parameters.odometry_noise,
            )
            .wrap_err("failed to predict pose filter")?;
            scored_state.score *= context
                .parameters
                .hypothesis_prediction_score_reduction_factor;
        }
        Ok(())
    }

    fn apply_measurements(
        &mut self,
        inputs: &CycleInputs,
        context: &mut CycleContext,
        measurement_noise: &MeasurementNoise,
    ) -> Result<FitErrorsPerMeasurement> {
        if !context.parameters.use_line_measurements || !inputs.line_measurements_allowed {
            return Ok(Vec::new());
        }
        let Some(line_data) = inputs.line_data.as_ref() else {
            return Ok(Vec::new());
        };

        let fit_errors = self.apply_measurement_batch(context, line_data, measurement_noise)?;
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
                    context.parameters.gradient_convergence_threshold,
                    context.parameters.gradient_descent_step_size,
                    context.parameters.line_length_acceptance_factor,
                    context
                        .parameters
                        .maximum_amount_of_gradient_descent_iterations,
                    context.parameters.maximum_amount_of_outer_iterations,
                    context.fit_errors.is_subscribed(),
                );

            Self::append_correspondence_lines_for_debug(context, &field_mark_correspondences);

            if field_mark_correspondences.is_empty() {
                continue;
            }

            if context.fit_errors.is_subscribed() {
                fit_errors_per_hypothesis.push(fit_errors);
            }

            let clamped_fit_error = fit_error.max(context.parameters.minimum_fit_error);
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

                if field_mark_correspondence.fit_error_sum()
                    < context.parameters.good_matching_threshold
                {
                    scored_state.score += context.parameters.score_per_good_match;
                }
            }

            scored_state.score += context.parameters.hypothesis_score_base_increase;
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
            scored_state.score >= context.parameters.hypothesis_retain_factor * best_score
        });

        context
            .fit_errors
            .fill_if_subscribed(|| fit_errors_per_measurement);

        Ok(ground_to_field.framed_transform())
    }

    fn get_best_hypothesis(&self) -> Option<&ScoredPose> {
        self.hypotheses
            .iter()
            .filter(|scored_filter| scored_filter.score.is_finite())
            .max_by(|left, right| left.score.total_cmp(&right.score))
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
    current_odometry_to_last_odometry: nalgebra::Isometry2<f32>,
    odometry_noise: &Vector3<f32>,
) -> Result<()> {
    let process_noise = odometry_process_noise(
        current_odometry_to_last_odometry,
        state.mean.z,
        odometry_noise,
    );

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

fn odometry_process_noise(
    current_odometry_to_last_odometry: nalgebra::Isometry2<f32>,
    current_orientation_angle: f32,
    odometry_noise: &Vector3<f32>,
) -> Matrix3<f32> {
    let odometry_translation = current_odometry_to_last_odometry.translation.vector;
    let translation_noise_in_odometry_frame = odometry_translation
        .abs()
        .component_mul(&odometry_noise.xy());
    let rotation_to_field = Rotation2::new(current_orientation_angle);
    let translation_process_noise = rotation_to_field.matrix()
        * Matrix2::from_diagonal(&translation_noise_in_odometry_frame)
        * rotation_to_field.matrix().transpose();

    let mut process_noise = Matrix3::zeros();
    process_noise
        .fixed_view_mut::<2, 2>(0, 0)
        .copy_from(&translation_process_noise);
    process_noise[(2, 2)] =
        current_odometry_to_last_odometry.rotation.angle().abs() * odometry_noise.z;
    process_noise
}

fn odometry_delta(last_odometer: Odometer, current_odometer: Odometer) -> nalgebra::Isometry2<f32> {
    let last_odometry_to_world = nalgebra::Isometry2::new(
        nalgebra::vector![last_odometer.x, last_odometer.y],
        last_odometer.theta,
    );
    let current_odometry_to_world = nalgebra::Isometry2::new(
        nalgebra::vector![current_odometer.x, current_odometer.y],
        current_odometer.theta,
    );

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
        let field_mark_correspondences = get_field_mark_correspondence(
            measured_lines_in_field,
            correction,
            field_marks,
            line_length_acceptance_factor,
        );
        let correspondence_points = get_correspondence_points(&field_mark_correspondences);
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
    let correspondence_points = get_correspondence_points(&field_mark_correspondences);
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
    let correction_transform = correction.framed_transform();
    measured_lines_in_field
        .iter()
        .filter_map(|&measured_line_in_field| {
            let transformed_line = correction_transform * measured_line_in_field;
            if !line_segment_is_finite(&transformed_line) {
                return None;
            }

            let (correspondences, _weight, field_mark, transformed_line) = field_marks
                .iter()
                .filter_map(|field_mark| {
                    let field_mark_length = match field_mark {
                        FieldMark::Line { line, .. } => line.length(),
                        FieldMark::Circle { radius, .. } => *radius,
                    };
                    if !field_mark_length.is_finite() || field_mark_length <= 0.0 {
                        return None;
                    }

                    let measured_line_length = transformed_line.length();
                    if !measured_line_length.is_finite()
                        || measured_line_length > field_mark_length * line_length_acceptance_factor
                    {
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

                    (weight.is_finite() && weight > 0.0).then_some((
                        correspondences,
                        weight,
                        field_mark,
                        transformed_line,
                    ))
                })
                .filter_map(
                    |(correspondence_points, weight, field_mark, transformed_line)| {
                        let distance_score = distance(
                            correspondence_points.correspondence_points.0.measured,
                            correspondence_points.correspondence_points.0.reference,
                        ) + distance(
                            correspondence_points.correspondence_points.1.measured,
                            correspondence_points.correspondence_points.1.reference,
                        );
                        let score = distance_score / weight;
                        score.is_finite().then_some((
                            score,
                            correspondence_points,
                            weight,
                            field_mark,
                            transformed_line,
                        ))
                    },
                )
                .min_by(|(left_score, ..), (right_score, ..)| left_score.total_cmp(right_score))
                .map(
                    |(_score, correspondence_points, weight, field_mark, transformed_line)| {
                        (correspondence_points, weight, field_mark, transformed_line)
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

fn line_segment_is_finite<Frame>(line: &LineSegment<Frame>) -> bool {
    line.0.x().is_finite()
        && line.0.y().is_finite()
        && line.1.x().is_finite()
        && line.1.y().is_finite()
}

fn get_correspondence_points(
    field_mark_correspondences: &[FieldMarkCorrespondence],
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
    use hsl_network_messages::PlayerNumber;
    use linear_algebra::Point2;
    use nalgebra::{matrix, vector};
    use types::players::Players;

    use super::*;

    fn test_parameters() -> Parameters {
        Parameters {
            circle_measurement_noise: vector![0.1, 0.1],
            good_matching_threshold: 0.1,
            gradient_convergence_threshold: 0.001,
            gradient_descent_step_size: 0.1,
            hypothesis_prediction_score_reduction_factor: 1.0,
            hypothesis_retain_factor: 0.5,
            hypothesis_score_base_increase: 1.0,
            initial_hypothesis_covariance: Matrix3::identity(),
            initial_hypothesis_score: 1.0,
            initial_poses: Players::new(InitialPose::default()),
            line_length_acceptance_factor: 1.5,
            line_measurement_noise: vector![0.1, 0.1],
            additional_moving_noise_line: vector![0.0, 0.0],
            additional_moving_noise_circle: vector![0.0, 0.0],
            maximum_amount_of_gradient_descent_iterations: 1,
            maximum_amount_of_outer_iterations: 1,
            minimum_fit_error: 0.0,
            odometry_noise: vector![0.0, 0.0, 0.0],
            penalized_distance: 0.5,
            penalized_hypothesis_covariance: Matrix3::identity(),
            score_per_good_match: 1.0,
            tentative_penalized_duration: Duration::from_secs(1),
            use_line_measurements: false,
            future_queue_lag: crate::FutureQueueLagParameters {
                odometer: Duration::ZERO,
                imu_state: Duration::ZERO,
                fall_down_state: Duration::ZERO,
                line_data: Duration::ZERO,
            },
        }
    }

    fn scored_pose(score: f32) -> ScoredPose {
        ScoredPose::from_isometry(
            Pose2::new(Point2::origin(), 0.0),
            Matrix3::identity(),
            score,
        )
    }

    fn empty_perception_input<T>() -> PerceptionInput<T> {
        PerceptionInput {
            persistent: BTreeMap::new(),
            temporary: BTreeMap::new(),
        }
    }

    #[test]
    fn best_hypothesis_ignores_nan_scores() {
        let localization = Localization {
            field_marks: Vec::new(),
            last_primary_state: PrimaryState::Safe,
            hypotheses: vec![scored_pose(f32::NAN), scored_pose(1.0)],
            hypotheses_when_entered_playing: Vec::new(),
            is_penalized_with_motion_in_set_or_initial: false,
            time_when_penalized_clicked: None,
            last_odometer: None,
            last_imu_state: None,
            last_fall_down_state: None,
            last_line_data_time: SystemTime::UNIX_EPOCH,
        };

        let best_hypothesis = localization
            .get_best_hypothesis()
            .expect("finite hypothesis is available");

        assert_eq!(best_hypothesis.score, 1.0);
    }

    #[test]
    fn cycle_does_not_report_converged_without_pose_output() {
        let parameters = test_parameters();
        let field_dimensions = FieldDimensions::SPL_2025;
        let player_number = PlayerNumber::One;
        let primary_state = PrimaryState::Safe;
        let mut localization = Localization {
            field_marks: Vec::new(),
            last_primary_state: PrimaryState::Safe,
            hypotheses: vec![scored_pose(1.0)],
            hypotheses_when_entered_playing: Vec::new(),
            is_penalized_with_motion_in_set_or_initial: false,
            time_when_penalized_clicked: None,
            last_odometer: None,
            last_imu_state: None,
            last_fall_down_state: None,
            last_line_data_time: SystemTime::UNIX_EPOCH,
        };
        let context = CycleContext {
            correspondence_lines: DebugOutput::new(false),
            fit_errors: DebugOutput::new(false),
            measured_lines_in_field: DebugOutput::new(false),
            pose_hypotheses: DebugOutput::new(false),
            updates: DebugOutput::new(false),
            gyro_movement: DebugOutput::new(false),
            filtered_game_controller_state: None,
            primary_state: &primary_state,
            cycle_start_time: SystemTime::UNIX_EPOCH,
            odometer: empty_perception_input(),
            fall_down_state: empty_perception_input(),
            imu_state: empty_perception_input(),
            line_data: empty_perception_input(),
            parameters: &parameters,
            field_dimensions: &field_dimensions,
            player_number: &player_number,
        };

        let outputs = localization.cycle(context).expect("cycle succeeds");

        assert!(outputs.ground_to_field.is_none());
        assert!(!outputs.is_localization_converged);
    }

    #[test]
    fn odometry_uses_cached_imu_when_batch_has_no_new_imu() {
        let parameters = test_parameters();
        let field_dimensions = FieldDimensions::SPL_2025;
        let player_number = PlayerNumber::One;
        let primary_state = PrimaryState::Playing;
        let mut localization = Localization::new(CreationContext {
            field_dimensions: &field_dimensions,
        })
        .expect("localization can be created");

        let mut initial_odometer = empty_perception_input();
        initial_odometer.persistent.insert(
            SystemTime::UNIX_EPOCH,
            vec![Odometer {
                x: 0.0,
                y: 0.0,
                theta: 0.0,
            }],
        );
        let mut initial_imu = empty_perception_input();
        initial_imu.persistent.insert(
            SystemTime::UNIX_EPOCH,
            vec![ImuState {
                roll_pitch_yaw: linear_algebra::vector![0.0, 0.0, 1.0],
                angular_velocity: linear_algebra::vector![0.0, 0.0, 0.0],
                linear_acceleration: linear_algebra::vector![0.0, 0.0, 0.0],
            }],
        );
        let initial_context = CycleContext {
            correspondence_lines: DebugOutput::new(false),
            fit_errors: DebugOutput::new(false),
            measured_lines_in_field: DebugOutput::new(false),
            pose_hypotheses: DebugOutput::new(false),
            updates: DebugOutput::new(false),
            gyro_movement: DebugOutput::new(false),
            filtered_game_controller_state: None,
            primary_state: &primary_state,
            cycle_start_time: SystemTime::UNIX_EPOCH,
            odometer: initial_odometer,
            fall_down_state: empty_perception_input(),
            imu_state: initial_imu,
            line_data: empty_perception_input(),
            parameters: &parameters,
            field_dimensions: &field_dimensions,
            player_number: &player_number,
        };
        localization.capture_cycle_inputs(&initial_context);

        let context_without_odometer = CycleContext {
            correspondence_lines: DebugOutput::new(false),
            fit_errors: DebugOutput::new(false),
            measured_lines_in_field: DebugOutput::new(false),
            pose_hypotheses: DebugOutput::new(false),
            updates: DebugOutput::new(false),
            gyro_movement: DebugOutput::new(false),
            filtered_game_controller_state: None,
            primary_state: &primary_state,
            cycle_start_time: SystemTime::UNIX_EPOCH + Duration::from_millis(5),
            odometer: empty_perception_input(),
            fall_down_state: empty_perception_input(),
            imu_state: empty_perception_input(),
            line_data: empty_perception_input(),
            parameters: &parameters,
            field_dimensions: &field_dimensions,
            player_number: &player_number,
        };
        localization.capture_cycle_inputs(&context_without_odometer);

        let mut odometer_without_imu = empty_perception_input();
        odometer_without_imu.persistent.insert(
            SystemTime::UNIX_EPOCH + Duration::from_millis(10),
            vec![Odometer {
                x: 1.0,
                y: 0.0,
                theta: 0.0,
            }],
        );
        let context_without_imu = CycleContext {
            correspondence_lines: DebugOutput::new(false),
            fit_errors: DebugOutput::new(false),
            measured_lines_in_field: DebugOutput::new(false),
            pose_hypotheses: DebugOutput::new(false),
            updates: DebugOutput::new(false),
            gyro_movement: DebugOutput::new(false),
            filtered_game_controller_state: None,
            primary_state: &primary_state,
            cycle_start_time: SystemTime::UNIX_EPOCH + Duration::from_millis(10),
            odometer: odometer_without_imu,
            fall_down_state: empty_perception_input(),
            imu_state: empty_perception_input(),
            line_data: empty_perception_input(),
            parameters: &parameters,
            field_dimensions: &field_dimensions,
            player_number: &player_number,
        };

        let inputs = localization.capture_cycle_inputs(&context_without_imu);

        assert_relative_eq!(
            inputs.current_odometry_to_last_odometry.rotation.angle(),
            0.0,
            epsilon = 0.0001
        );
        assert_relative_eq!(
            inputs.current_odometry_to_last_odometry.translation.vector,
            nalgebra::vector![1.0_f32.cos(), -1.0_f32.sin()],
            epsilon = 0.0001
        );
    }

    #[test]
    fn field_mark_correspondence_ignores_nan_distance_scores() {
        let correspondences = get_field_mark_correspondence(
            &[LineSegment(point![f32::NAN, 0.0], point![1.0, 0.0])],
            nalgebra::Isometry2::identity(),
            &[FieldMark::Line {
                line: LineSegment(point![0.0, 0.0], point![1.0, 0.0]),
                direction: Direction::PositiveX,
            }],
            1.5,
        );

        assert!(correspondences.is_empty());
    }

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
        let delta = odometer.to(last_odometer);

        assert_relative_eq!(delta.translation().x(), 1.0, epsilon = 0.0001);
        assert_relative_eq!(delta.translation().y(), 0.0, epsilon = 0.0001);
        assert_relative_eq!(delta.orientation().angle(), 0.2, epsilon = 0.0001);
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
        let delta = odometer.to(last_odometer);

        assert_relative_eq!(delta.orientation().angle(), -PI + 0.2, epsilon = 0.0001);
    }

    #[test]
    fn standing_still_adds_no_odometry_process_noise() {
        let process_noise = odometry_process_noise(
            nalgebra::Isometry2::identity(),
            0.3,
            &vector![0.02, 0.02, 0.001],
        );

        assert_relative_eq!(process_noise, Matrix3::zeros(), epsilon = 0.0001);
    }

    #[test]
    fn rotational_process_noise_scales_with_actual_turn() {
        let process_noise = odometry_process_noise(
            nalgebra::Isometry2::new(vector![0.0, 0.0], 0.2),
            0.0,
            &vector![0.02, 0.02, 0.001],
        );

        assert_relative_eq!(
            process_noise,
            matrix![
                0.0, 0.0, 0.0;
                0.0, 0.0, 0.0;
                0.0, 0.0, 0.0002
            ],
            epsilon = 0.0001
        );
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
