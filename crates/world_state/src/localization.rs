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
use linear_algebra::{IntoTransform, Isometry2, Pose2, point};
use nalgebra::{Matrix2, Matrix3, Rotation2, UnitComplex, Vector2, Vector3, matrix, vector};
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
    field_marks::{CorrespondencePoints, FieldMark, field_marks_from_field_dimensions},
    filtered_game_controller_state::FilteredGameControllerState,
    initial_pose::InitialPose,
    line_data::LineData,
    localization::{
        CandidateAlternativeDebug, LocalizationDebugFrame, LocalizationDebugHypothesis,
        LocalizationMatchDebug, MeasuredLineDebug, MeasuredLineRejectionReason, MeasuredLineStatus,
        ScoredPose,
    },
    multivariate_normal_distribution::MultivariateNormalDistribution,
    players::Players,
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
    time_when_penalized_clicked: Option<SystemTime>,
    last_odometer: Option<Odometer>,
    last_line_data_time: SystemTime,
}

#[context]
pub struct CreationContext {
    field_dimensions: Parameter<FieldDimensions, "field_dimensions">,
}

#[context]
pub struct CycleContext {
    debug_frame: AdditionalOutput<LocalizationDebugFrame, "localization.debug_frame">,

    filtered_game_controller_state:
        Input<Option<FilteredGameControllerState>, "filtered_game_controller_state?">,
    primary_state: Input<PrimaryState, "primary_state">,
    cycle_time: Input<CycleTime, "cycle_time">,

    odometer: PerceptionInput<Odometer, "Odometry", "odometer">,
    fall_down_state: PerceptionInput<Option<FallDownState>, "FallDownState", "fall_down_state?">,
    imu_state: PerceptionInput<ImuState, "Motion", "imu_state">,
    line_data: PerceptionInput<Option<LineData>, "Vision", "line_data?">,

    field_dimensions: Parameter<FieldDimensions, "field_dimensions">,
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
    ambiguity_margin_threshold: Parameter<f32, "localization.ambiguity_margin_threshold">,
    locality_rotation_scale: Parameter<f32, "localization.locality_rotation_scale">,
    locality_translation_scale: Parameter<f32, "localization.locality_translation_scale">,
    locality_weight: Parameter<f32, "localization.locality_weight">,
    maximum_amount_of_gradient_descent_iterations:
        Parameter<usize, "localization.maximum_amount_of_gradient_descent_iterations">,
    minimum_inlier_count: Parameter<usize, "localization.minimum_inlier_count">,
    minimum_fit_error: Parameter<f32, "localization.minimum_fit_error">,
    minimum_measurement_confidence: Parameter<f32, "localization.minimum_measurement_confidence">,
    odometry_noise: Parameter<Vector3<f32>, "localization.odometry_noise">,
    player_number: Parameter<PlayerNumber, "player_number">,
    penalized_distance: Parameter<f32, "localization.penalized_distance">,
    penalized_hypothesis_covariance:
        Parameter<Matrix3<f32>, "localization.penalized_hypothesis_covariance">,
    pose_measurement_noise: Parameter<Matrix3<f32>, "localization.pose_measurement_noise">,
    ransac_inlier_threshold: Parameter<f32, "localization.ransac_inlier_threshold">,
    ransac_iterations: Parameter<usize, "localization.ransac_iterations">,
    tentative_penalized_duration: Parameter<Duration, "localization.tentative_penalized_duration">,
    use_line_measurements: Parameter<bool, "localization.use_line_measurements">,
}

#[context]
#[derive(Default)]
pub struct MainOutputs {
    pub ground_to_field: MainOutput<Option<Isometry2<Ground, Field>>>,
    pub is_localization_converged: MainOutput<bool>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum PenaltyExitStrategy {
    KeepCurrent,
    RestorePlaying,
    ResetToPenalized,
}

struct LocalityParameters {
    translation_scale: f32,
    rotation_scale: f32,
    weight: f32,
}

struct MatchingParameters {
    ambiguity_margin_threshold: f32,
    gradient_convergence_threshold: f32,
    gradient_descent_step_size: f32,
    line_length_acceptance_factor: f32,
    locality: LocalityParameters,
    maximum_amount_of_gradient_descent_iterations: usize,
    minimum_fit_error: f32,
    minimum_inlier_count: usize,
    minimum_measurement_confidence: f32,
    ransac_inlier_threshold: f32,
    ransac_iterations: usize,
}

#[derive(Clone)]
struct BatchCorrespondenceResult {
    correction: nalgebra::Isometry2<f32>,
    field_mark_correspondences: Vec<FieldMarkCorrespondence>,
    fit_error: f32,
    global_consensus: f32,
    local_ambiguity: f32,
    locality_confidence: f32,
    matched_lines: usize,
    measurement_confidence: f32,
    observability_covariance: Matrix3<f32>,
    observability_variances: Vector3<f32>,
    final_matches: Vec<LocalizationMatchDebug>,
    candidate_summaries: Vec<MeasuredLineDebug>,
    total_cost: f32,
    unmatched_lines: usize,
}

#[derive(Clone, Copy)]
struct LineCandidate {
    field_mark_correspondence: FieldMarkCorrespondence,
    geometric_cost: f32,
    locality_cost: f32,
    total_cost: f32,
}

#[derive(Clone)]
struct CandidateSeed {
    measured_line_index: usize,
    line_candidate: LineCandidate,
    seed_cost: f32,
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
            field_marks: field_marks_from_field_dimensions(context.field_dimensions),
            last_primary_state: PrimaryState::Safe,
            hypotheses: Vec::new(),
            hypotheses_when_entered_playing: Vec::new(),
            is_penalized_with_motion_in_set_or_initial: false,
            time_when_penalized_clicked: None,
            last_odometer: None,
            last_line_data_time: SystemTime::UNIX_EPOCH,
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

        let imu_state = Self::latest_imu_state(context);
        let gyro_movement = imu_state.angular_velocity.norm();

        let fall_down_state = Self::latest_fall_down_state(context);
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
        self.last_odometer = odometer_with_imu_yaw;

        let line_data = context
            .line_data
            .persistent
            .iter()
            .chain(&context.line_data.temporary)
            .filter(|(time, _)| **time > self.last_line_data_time)
            .flat_map(|(time, detections)| {
                Some((*time, (*detections.iter().flatten().last()?).clone()))
            })
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
        let matching_parameters = self.matching_parameters(context);
        self.predict_hypotheses(context, inputs.current_odometry_to_last_odometry)?;
        let debug_hypotheses = self.apply_measurements(inputs, context, &matching_parameters)?;
        self.finalize_hypotheses(context, inputs, debug_hypotheses)
    }

    fn matching_parameters(&self, context: &CycleContext) -> MatchingParameters {
        MatchingParameters {
            ambiguity_margin_threshold: (*context.ambiguity_margin_threshold).max(f32::EPSILON),
            gradient_convergence_threshold: *context.gradient_convergence_threshold,
            gradient_descent_step_size: *context.gradient_descent_step_size,
            line_length_acceptance_factor: *context.line_length_acceptance_factor,
            locality: LocalityParameters {
                translation_scale: (*context.locality_translation_scale).max(f32::EPSILON),
                rotation_scale: (*context.locality_rotation_scale).max(f32::EPSILON),
                weight: *context.locality_weight,
            },
            maximum_amount_of_gradient_descent_iterations: *context
                .maximum_amount_of_gradient_descent_iterations,
            minimum_fit_error: *context.minimum_fit_error,
            minimum_inlier_count: *context.minimum_inlier_count,
            minimum_measurement_confidence: *context.minimum_measurement_confidence,
            ransac_inlier_threshold: *context.ransac_inlier_threshold,
            ransac_iterations: *context.ransac_iterations,
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
                context.odometry_noise,
            )
            .wrap_err("failed to predict pose filter")?;
            scored_state.score *= *context.hypothesis_prediction_score_reduction_factor;
        }
        Ok(())
    }

    fn apply_measurements(
        &mut self,
        inputs: &CycleInputs,
        context: &mut CycleContext,
        matching_parameters: &MatchingParameters,
    ) -> Result<Vec<LocalizationDebugHypothesis>> {
        if !*context.use_line_measurements || !inputs.line_measurements_allowed {
            return Ok(self
                .hypotheses
                .iter()
                .map(|scored_state| {
                    empty_debug_hypothesis(
                        scored_state,
                        scored_state.state.as_isometry().framed_transform(),
                    )
                })
                .collect());
        }
        let Some(line_data) = inputs.line_data.as_ref() else {
            return Ok(self
                .hypotheses
                .iter()
                .map(|scored_state| {
                    empty_debug_hypothesis(
                        scored_state,
                        scored_state.state.as_isometry().framed_transform(),
                    )
                })
                .collect());
        };

        self.apply_measurement_batch(context, line_data, matching_parameters)
    }

    fn apply_measurement_batch(
        &mut self,
        context: &mut CycleContext,
        line_data: &LineData,
        matching_parameters: &MatchingParameters,
    ) -> Result<Vec<LocalizationDebugHypothesis>> {
        let mut debug_hypotheses = Vec::with_capacity(self.hypotheses.len());

        for scored_state in &mut self.hypotheses {
            let ground_to_field: Isometry2<Ground, Field> =
                scored_state.state.as_isometry().framed_transform();
            let measured_lines_in_field: Vec<_> = line_data
                .lines
                .iter()
                .map(|&measured_line_in_ground| ground_to_field * measured_line_in_ground)
                .collect();

            if measured_lines_in_field.is_empty() {
                debug_hypotheses.push(empty_debug_hypothesis(scored_state, ground_to_field));
                continue;
            }

            let Some(batch_result) = get_fitted_field_mark_correspondence(
                &measured_lines_in_field,
                &self.field_marks,
                matching_parameters,
            ) else {
                debug_hypotheses.push(rejected_debug_hypothesis(
                    scored_state,
                    ground_to_field,
                    &measured_lines_in_field,
                    &self.field_marks,
                    matching_parameters,
                ));
                continue;
            };

            let clamped_fit_error = batch_result
                .fit_error
                .max(matching_parameters.minimum_fit_error);
            let corrected_ground_to_field: Isometry2<Ground, Field> =
                (batch_result.correction * ground_to_field.inner).framed_transform();
            let measurement = vector![
                corrected_ground_to_field.translation().x(),
                corrected_ground_to_field.translation().y(),
                corrected_ground_to_field.orientation().angle(),
            ];
            let measurement_covariance = measurement_covariance_from_correspondences(
                *context.pose_measurement_noise,
                &batch_result.field_mark_correspondences,
                batch_result.correction,
                clamped_fit_error / batch_result.measurement_confidence,
            );

            scored_state
                .state
                .update_with_3d_pose(measurement, measurement_covariance, |state| state)
                .context("failed to update pose filter with fused localization measurement")?;

            scored_state.score += *context.hypothesis_score_base_increase
                + batch_result.matched_lines as f32 * batch_result.measurement_confidence;

            debug_hypotheses.push(successful_debug_hypothesis(
                scored_state,
                corrected_ground_to_field,
                &batch_result,
            ));
        }

        Ok(debug_hypotheses)
    }

    fn finalize_hypotheses(
        &mut self,
        context: &mut CycleContext,
        inputs: &CycleInputs,
        debug_hypotheses: Vec<LocalizationDebugHypothesis>,
    ) -> Result<Isometry2<Ground, Field>> {
        let best_hypothesis = self
            .get_best_hypothesis()
            .ok_or_eyre("localization has no pose hypotheses after update")?;
        let best_score = best_hypothesis.score;
        let ground_to_field = best_hypothesis.state.as_isometry();
        let retain_threshold = *context.hypothesis_retain_factor * best_score;
        let retained = self
            .hypotheses
            .drain(..)
            .zip(debug_hypotheses)
            .filter(|(scored_state, _)| scored_state.score >= retain_threshold)
            .collect::<Vec<_>>();
        self.hypotheses = retained
            .iter()
            .map(|(scored_state, _)| *scored_state)
            .collect();
        let debug_hypotheses = retained
            .into_iter()
            .map(|(_, debug_hypothesis)| debug_hypothesis)
            .collect::<Vec<_>>();

        context.debug_frame.fill_if_subscribed(|| {
            let best_hypothesis_index = debug_hypotheses
                .iter()
                .enumerate()
                .max_by_key(|(_, hypothesis)| NotNan::new(hypothesis.score).unwrap())
                .map(|(index, _)| index);
            let measured_lines_in_field = best_hypothesis_index
                .and_then(|index| debug_hypotheses.get(index))
                .map(|hypothesis| {
                    hypothesis
                        .candidate_summaries
                        .iter()
                        .map(|summary| summary.measured_line)
                        .collect()
                })
                .unwrap_or_default();

            LocalizationDebugFrame {
                cycle_start_time: unix_duration(inputs.cycle_start_time),
                gyro_movement: inputs.gyro_movement,
                best_hypothesis_index,
                measured_lines_in_field,
                hypotheses: debug_hypotheses,
            }
        });

        Ok(ground_to_field.framed_transform())
    }

    fn compose_main_outputs(
        &self,
        _inputs: &CycleInputs,
        _context: &mut CycleContext,
        ground_to_field: Option<Isometry2<Ground, Field>>,
    ) -> MainOutputs {
        let is_localization_converged = self.hypotheses.len() == 1;

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

fn empty_debug_hypothesis(
    scored_state: &ScoredPose,
    ground_to_field: Isometry2<Ground, Field>,
) -> LocalizationDebugHypothesis {
    LocalizationDebugHypothesis {
        ground_to_field,
        score: scored_state.score,
        covariance: scored_state.state.covariance,
        correction_delta: Vector3::zeros(),
        fit_error: f32::INFINITY,
        matched_lines: 0,
        unmatched_lines: 0,
        global_consensus: 0.0,
        local_ambiguity: 0.0,
        locality_confidence: 1.0,
        measurement_confidence: 0.0,
        observability_covariance: Matrix3::identity() * 1.0e6,
        observability_variances: vector![1.0e6, 1.0e6, 1.0e6],
        final_matches: Vec::new(),
        candidate_summaries: Vec::new(),
    }
}

fn rejected_debug_hypothesis(
    scored_state: &ScoredPose,
    ground_to_field: Isometry2<Ground, Field>,
    measured_lines_in_field: &[LineSegment<Field>],
    field_marks: &[FieldMark],
    matching_parameters: &MatchingParameters,
) -> LocalizationDebugHypothesis {
    let evaluation = summarize_correction(
        measured_lines_in_field,
        field_marks,
        nalgebra::Isometry2::identity(),
        matching_parameters,
    );

    LocalizationDebugHypothesis {
        unmatched_lines: measured_lines_in_field.len(),
        candidate_summaries: evaluation.candidate_summaries,
        ..empty_debug_hypothesis(scored_state, ground_to_field)
    }
}

fn successful_debug_hypothesis(
    scored_state: &ScoredPose,
    ground_to_field: Isometry2<Ground, Field>,
    batch_result: &BatchCorrespondenceResult,
) -> LocalizationDebugHypothesis {
    LocalizationDebugHypothesis {
        ground_to_field,
        score: scored_state.score,
        covariance: scored_state.state.covariance,
        correction_delta: vector![
            batch_result.correction.translation.vector.x,
            batch_result.correction.translation.vector.y,
            batch_result.correction.rotation.angle(),
        ],
        fit_error: batch_result.fit_error,
        matched_lines: batch_result.matched_lines,
        unmatched_lines: batch_result.unmatched_lines,
        global_consensus: batch_result.global_consensus,
        local_ambiguity: batch_result.local_ambiguity,
        locality_confidence: batch_result.locality_confidence,
        measurement_confidence: batch_result.measurement_confidence,
        observability_covariance: batch_result.observability_covariance,
        observability_variances: batch_result.observability_variances,
        final_matches: batch_result.final_matches.clone(),
        candidate_summaries: batch_result.candidate_summaries.clone(),
    }
}

fn unix_duration(system_time: SystemTime) -> Duration {
    system_time
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap_or_default()
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

#[derive(Clone, Copy, Debug)]
#[allow(dead_code)]
pub struct FieldMarkCorrespondence {
    measured_line_index: usize,
    measured_line_in_field: LineSegment<Field>,
    field_mark: FieldMark,
    pub correspondence_points: (CorrespondencePoints, CorrespondencePoints),
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

fn get_fitted_field_mark_correspondence(
    measured_lines_in_field: &[LineSegment<Field>],
    field_marks: &[FieldMark],
    matching_parameters: &MatchingParameters,
) -> Option<BatchCorrespondenceResult> {
    if measured_lines_in_field.is_empty() || field_marks.is_empty() {
        return None;
    }

    let candidate_seeds =
        generate_candidate_seeds(measured_lines_in_field, field_marks, matching_parameters);
    let initial_correction = find_best_ransac_correction(
        measured_lines_in_field,
        field_marks,
        &candidate_seeds,
        matching_parameters,
    )?
    .correction;
    let mut correction = initial_correction;

    for _ in 0..matching_parameters.maximum_amount_of_gradient_descent_iterations {
        let evaluation = evaluate_correction(
            measured_lines_in_field,
            field_marks,
            correction,
            matching_parameters,
        )?;
        let correspondence_points =
            get_correspondence_points(&evaluation.field_mark_correspondences);
        let weight_matrices = weight_matrices(&correspondence_points, correction);

        let translation_gradient: Vector2<f32> = correspondence_points
            .iter()
            .zip(weight_matrices.iter())
            .map(|(correspondence_points, weight_matrix)| {
                2.0 * weight_matrix
                    * ((correction * correspondence_points.measured.inner)
                        - correspondence_points.reference.inner)
            })
            .sum::<Vector2<f32>>()
            / correspondence_points.len() as f32
            + locality_translation_gradient(correction, &matching_parameters.locality);
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
            / correspondence_points.len() as f32
            + locality_rotation_gradient(correction, &matching_parameters.locality);

        let gradient_norm = vector![
            translation_gradient.x,
            translation_gradient.y,
            rotation_gradient
        ]
        .norm();
        if gradient_norm < matching_parameters.gradient_convergence_threshold {
            break;
        }

        correction = nalgebra::Isometry2::new(
            correction.translation.vector
                - (matching_parameters.gradient_descent_step_size * translation_gradient),
            rotation - matching_parameters.gradient_descent_step_size * rotation_gradient,
        );
    }

    let batch_result = evaluate_correction(
        measured_lines_in_field,
        field_marks,
        correction,
        matching_parameters,
    )?;

    Some(batch_result)
}

fn generate_candidate_seeds(
    measured_lines_in_field: &[LineSegment<Field>],
    field_marks: &[FieldMark],
    matching_parameters: &MatchingParameters,
) -> Vec<CandidateSeed> {
    let mut candidate_seeds = measured_lines_in_field
        .iter()
        .enumerate()
        .flat_map(|(measured_line_index, &measured_line_in_field)| {
            get_line_candidates(
                measured_line_index,
                measured_line_in_field,
                nalgebra::Isometry2::identity(),
                field_marks,
                matching_parameters.line_length_acceptance_factor,
                &matching_parameters.locality,
            )
            .into_iter()
            .filter_map(move |line_candidate| {
                let correspondence_points = get_correspondence_points(std::slice::from_ref(
                    &line_candidate.field_mark_correspondence,
                ));
                let delta_correction =
                    estimate_correction_from_correspondence_points(&correspondence_points)?;
                Some(CandidateSeed {
                    measured_line_index,
                    line_candidate,
                    seed_cost: line_candidate.geometric_cost
                        + matching_parameters.locality.weight
                            * locality_cost(delta_correction, &matching_parameters.locality),
                })
            })
        })
        .collect::<Vec<_>>();

    candidate_seeds.sort_by_key(|candidate| NotNan::new(candidate.seed_cost).unwrap());
    candidate_seeds
}

fn find_best_ransac_correction(
    measured_lines_in_field: &[LineSegment<Field>],
    field_marks: &[FieldMark],
    candidate_seeds: &[CandidateSeed],
    matching_parameters: &MatchingParameters,
) -> Option<BatchCorrespondenceResult> {
    let mut candidate_corrections = vec![nalgebra::Isometry2::identity()];
    let top_seed_count = candidate_seeds.len().min(
        matching_parameters
            .ransac_iterations
            .saturating_mul(2)
            .max(1),
    );

    for seed in candidate_seeds.iter().take(top_seed_count) {
        let correspondence_points = get_correspondence_points(std::slice::from_ref(
            &seed.line_candidate.field_mark_correspondence,
        ));
        if let Some(correction) =
            estimate_correction_from_correspondence_points(&correspondence_points)
        {
            candidate_corrections.push(correction);
        }
    }

    'sample_pairs: for (first_index, first_seed) in
        candidate_seeds.iter().take(top_seed_count).enumerate()
    {
        for second_seed in candidate_seeds
            .iter()
            .take(top_seed_count)
            .skip(first_index + 1)
        {
            if first_seed.measured_line_index == second_seed.measured_line_index {
                continue;
            }
            let sample = [
                first_seed.line_candidate.field_mark_correspondence,
                second_seed.line_candidate.field_mark_correspondence,
            ];
            let correspondence_points = get_correspondence_points(&sample);
            if let Some(correction) =
                estimate_correction_from_correspondence_points(&correspondence_points)
            {
                candidate_corrections.push(correction);
            }
            if candidate_corrections.len() >= matching_parameters.ransac_iterations.max(1) {
                break 'sample_pairs;
            }
        }
    }

    candidate_corrections
        .into_iter()
        .filter_map(|correction| {
            evaluate_correction(
                measured_lines_in_field,
                field_marks,
                correction,
                matching_parameters,
            )
        })
        .max_by(compare_batch_results)
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

fn measurement_covariance_from_correspondences(
    base_covariance: Matrix3<f32>,
    field_mark_correspondences: &[FieldMarkCorrespondence],
    correction: nalgebra::Isometry2<f32>,
    scale: f32,
) -> Matrix3<f32> {
    let correspondence_points = get_correspondence_points(field_mark_correspondences);
    let weight_matrices = weight_matrices(&correspondence_points, correction);
    let information =
        geometry_information_matrix(&correspondence_points, &weight_matrices, correction);
    anisotropic_covariance_from_information(base_covariance, information, scale)
}

fn geometry_information_matrix(
    correspondence_points: &[CorrespondencePoints],
    weight_matrices: &[Matrix2<f32>],
    correction: nalgebra::Isometry2<f32>,
) -> Matrix3<f32> {
    if correspondence_points.is_empty() {
        return Matrix3::zeros();
    }

    let rotation = correction.rotation.angle();
    let rotation_derivative =
        matrix![-rotation.sin(), -rotation.cos(); rotation.cos(), -rotation.sin()];

    correspondence_points
        .iter()
        .zip(weight_matrices.iter())
        .map(|(correspondence_point, weight_matrix)| {
            let rotated_point_derivative =
                rotation_derivative * correspondence_point.measured.inner.coords;
            let jacobian = nalgebra::Matrix2x3::new(
                1.0,
                0.0,
                rotated_point_derivative.x,
                0.0,
                1.0,
                rotated_point_derivative.y,
            );
            jacobian.transpose() * weight_matrix * jacobian
        })
        .sum::<Matrix3<f32>>()
        / correspondence_points.len() as f32
}

fn anisotropic_covariance_from_information(
    base_covariance: Matrix3<f32>,
    information: Matrix3<f32>,
    scale: f32,
) -> Matrix3<f32> {
    let minimum_information = 1.0e-3;
    let symmetric_information = 0.5 * (information + information.transpose());
    let eigen = symmetric_information.symmetric_eigen();

    let diagonal = Matrix3::from_diagonal(&Vector3::new(
        projected_variance(
            &base_covariance,
            &eigen.eigenvectors.column(0).into_owned(),
            scale / eigen.eigenvalues[0].max(minimum_information),
        ),
        projected_variance(
            &base_covariance,
            &eigen.eigenvectors.column(1).into_owned(),
            scale / eigen.eigenvalues[1].max(minimum_information),
        ),
        projected_variance(
            &base_covariance,
            &eigen.eigenvectors.column(2).into_owned(),
            scale / eigen.eigenvalues[2].max(minimum_information),
        ),
    ));

    eigen.eigenvectors * diagonal * eigen.eigenvectors.transpose()
}

fn projected_variance(base_covariance: &Matrix3<f32>, direction: &Vector3<f32>, scale: f32) -> f32 {
    let base_variance = (direction.transpose() * base_covariance * direction).x;
    base_variance.max(f32::EPSILON) * scale
}

fn compare_batch_results(
    left: &BatchCorrespondenceResult,
    right: &BatchCorrespondenceResult,
) -> std::cmp::Ordering {
    left.matched_lines
        .cmp(&right.matched_lines)
        .then_with(|| {
            NotNan::new(right.total_cost)
                .unwrap()
                .cmp(&NotNan::new(left.total_cost).unwrap())
        })
        .then_with(|| {
            NotNan::new(right.fit_error)
                .unwrap()
                .cmp(&NotNan::new(left.fit_error).unwrap())
        })
}

struct CorrectionEvaluation {
    field_mark_correspondences: Vec<FieldMarkCorrespondence>,
    final_matches: Vec<LocalizationMatchDebug>,
    candidate_summaries: Vec<MeasuredLineDebug>,
    fit_error_sum: f32,
    ambiguity_sum: f32,
    total_cost_sum: f32,
}

fn evaluate_correction(
    measured_lines_in_field: &[LineSegment<Field>],
    field_marks: &[FieldMark],
    correction: nalgebra::Isometry2<f32>,
    matching_parameters: &MatchingParameters,
) -> Option<BatchCorrespondenceResult> {
    let evaluation = summarize_correction(
        measured_lines_in_field,
        field_marks,
        correction,
        matching_parameters,
    );
    if evaluation.field_mark_correspondences.len() < matching_parameters.minimum_inlier_count {
        return None;
    }

    let matched_lines = evaluation.field_mark_correspondences.len();
    let unmatched_lines = measured_lines_in_field.len().saturating_sub(matched_lines);
    let fit_error = evaluation.fit_error_sum / matched_lines as f32;
    let global_consensus = matched_lines as f32 / measured_lines_in_field.len() as f32;
    let local_ambiguity = evaluation.ambiguity_sum / matched_lines as f32;
    let locality_cost = locality_cost(correction, &matching_parameters.locality);
    let locality_confidence = 1.0 / (1.0 + locality_cost.sqrt());
    let measurement_confidence = (global_consensus * local_ambiguity * locality_confidence)
        .clamp(matching_parameters.minimum_measurement_confidence, 1.0);
    let total_cost = evaluation.total_cost_sum / matched_lines as f32;
    let observability_covariance = measurement_covariance_from_correspondences(
        Matrix3::identity(),
        &evaluation.field_mark_correspondences,
        correction,
        1.0,
    );
    let observability_variances = covariance_principal_variances(observability_covariance);

    Some(BatchCorrespondenceResult {
        correction,
        field_mark_correspondences: evaluation.field_mark_correspondences,
        fit_error,
        global_consensus,
        local_ambiguity,
        locality_confidence,
        matched_lines,
        measurement_confidence,
        observability_covariance,
        observability_variances,
        final_matches: evaluation.final_matches,
        candidate_summaries: evaluation.candidate_summaries,
        total_cost,
        unmatched_lines,
    })
}

fn summarize_correction(
    measured_lines_in_field: &[LineSegment<Field>],
    field_marks: &[FieldMark],
    correction: nalgebra::Isometry2<f32>,
    matching_parameters: &MatchingParameters,
) -> CorrectionEvaluation {
    let mut field_mark_correspondences = Vec::new();
    let mut final_matches = Vec::new();
    let mut candidate_summaries = Vec::with_capacity(measured_lines_in_field.len());
    let mut fit_error_sum = 0.0;
    let mut ambiguity_sum = 0.0;
    let mut total_cost_sum = 0.0;

    for (measured_line_index, &measured_line_in_field) in measured_lines_in_field.iter().enumerate()
    {
        let mut candidates = get_line_candidates(
            measured_line_index,
            measured_line_in_field,
            correction,
            field_marks,
            matching_parameters.line_length_acceptance_factor,
            &matching_parameters.locality,
        );
        candidates.sort_by_key(|candidate| NotNan::new(candidate.total_cost).unwrap());

        let (status, selected_field_mark, rejection_reason) = match candidates.first() {
            None => (
                MeasuredLineStatus::NoCandidate,
                None,
                Some(MeasuredLineRejectionReason::NoCandidate),
            ),
            Some(best_candidate)
                if best_candidate.total_cost > matching_parameters.ransac_inlier_threshold =>
            {
                (
                    MeasuredLineStatus::RejectedByThreshold,
                    None,
                    Some(MeasuredLineRejectionReason::TotalCostAboveThreshold),
                )
            }
            Some(best_candidate) => {
                let best_total_cost = best_candidate.total_cost;
                let ambiguity_margin = candidates
                    .get(1)
                    .map(|second_candidate| {
                        ((second_candidate.total_cost - best_total_cost)
                            / matching_parameters.ambiguity_margin_threshold)
                            .clamp(0.0, 1.0)
                    })
                    .unwrap_or(1.0);

                ambiguity_sum += ambiguity_margin;
                fit_error_sum += best_candidate.geometric_cost;
                total_cost_sum += best_total_cost;
                field_mark_correspondences.push(best_candidate.field_mark_correspondence);
                final_matches.push(LocalizationMatchDebug {
                    measured_line_index,
                    measured_line: measured_line_in_field,
                    field_mark: best_candidate.field_mark_correspondence.field_mark,
                    correspondence_points: best_candidate
                        .field_mark_correspondence
                        .correspondence_points,
                    geometric_cost: best_candidate.geometric_cost,
                    locality_cost: best_candidate.locality_cost,
                    total_cost: best_candidate.total_cost,
                    ambiguity_margin,
                });
                (
                    MeasuredLineStatus::Matched,
                    Some(best_candidate.field_mark_correspondence.field_mark),
                    None,
                )
            }
        };

        candidate_summaries.push(MeasuredLineDebug {
            measured_line_index,
            measured_line: measured_line_in_field,
            status,
            selected_field_mark,
            best_geometric_cost: candidates.first().map(|candidate| candidate.geometric_cost),
            best_locality_cost: candidates.first().map(|candidate| candidate.locality_cost),
            best_total_cost: candidates.first().map(|candidate| candidate.total_cost),
            inlier_threshold: matching_parameters.ransac_inlier_threshold,
            rejection_reason,
            candidates: candidates
                .iter()
                .take(3)
                .enumerate()
                .map(|(candidate_index, candidate)| CandidateAlternativeDebug {
                    field_mark: candidate.field_mark_correspondence.field_mark,
                    geometric_cost: candidate.geometric_cost,
                    locality_cost: candidate.locality_cost,
                    total_cost: candidate.total_cost,
                    accepted: candidate_index == 0 && status == MeasuredLineStatus::Matched,
                })
                .collect(),
        });
    }

    CorrectionEvaluation {
        field_mark_correspondences,
        final_matches,
        candidate_summaries,
        fit_error_sum,
        ambiguity_sum,
        total_cost_sum,
    }
}

fn get_line_candidates(
    measured_line_index: usize,
    measured_line: LineSegment<Field>,
    correction: nalgebra::Isometry2<f32>,
    field_marks: &[FieldMark],
    line_length_acceptance_factor: f32,
    locality_parameters: &LocalityParameters,
) -> Vec<LineCandidate> {
    field_marks
        .iter()
        .filter_map(|&field_mark| {
            let transformed_line = correction.framed_transform() * measured_line;
            let field_mark_length = match field_mark {
                FieldMark::Line { line, .. } => line.length(),
                FieldMark::Circle { radius, .. } => radius,
            };
            if field_mark_length <= 0.0 {
                return None;
            }

            let measured_line_length = transformed_line.length();
            if measured_line_length > field_mark_length * line_length_acceptance_factor {
                return None;
            }

            let correspondences = field_mark.to_correspondence_points(transformed_line);
            let inverse_transformation = correction.inverse().framed_transform();
            let field_mark_correspondence = FieldMarkCorrespondence {
                measured_line_index,
                measured_line_in_field: inverse_transformation * transformed_line,
                field_mark,
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
            };
            let correspondence_points =
                get_correspondence_points(std::slice::from_ref(&field_mark_correspondence));
            let geometric_cost = get_fit_error(
                &correspondence_points,
                &weight_matrices(&correspondence_points, correction),
                correction,
            );
            let candidate_correction =
                estimate_correction_from_correspondence_points(&correspondence_points)
                    .unwrap_or(correction);
            let locality_cost = locality_cost(candidate_correction, locality_parameters);

            Some(LineCandidate {
                field_mark_correspondence,
                geometric_cost,
                locality_cost,
                total_cost: geometric_cost + locality_parameters.weight * locality_cost,
            })
        })
        .collect()
}

fn covariance_principal_variances(covariance: Matrix3<f32>) -> Vector3<f32> {
    let symmetric_covariance = 0.5 * (covariance + covariance.transpose());
    let eigen = symmetric_covariance.symmetric_eigen();
    Vector3::new(
        eigen.eigenvalues[0],
        eigen.eigenvalues[1],
        eigen.eigenvalues[2],
    )
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

fn estimate_correction_from_correspondence_points(
    correspondence_points: &[CorrespondencePoints],
) -> Option<nalgebra::Isometry2<f32>> {
    if correspondence_points.len() < 2 {
        return None;
    }

    let measured_centroid = correspondence_points
        .iter()
        .map(|point| point.measured.inner.coords)
        .sum::<Vector2<f32>>()
        / correspondence_points.len() as f32;
    let reference_centroid = correspondence_points
        .iter()
        .map(|point| point.reference.inner.coords)
        .sum::<Vector2<f32>>()
        / correspondence_points.len() as f32;
    let covariance = correspondence_points
        .iter()
        .fold(Matrix2::zeros(), |sum, point| {
            let measured = point.measured.inner.coords - measured_centroid;
            let reference = point.reference.inner.coords - reference_centroid;
            sum + measured * reference.transpose()
        });
    let svd = covariance.svd(true, true);
    let u = svd.u?;
    let v_t = svd.v_t?;
    let mut rotation_matrix = v_t.transpose() * u.transpose();
    if rotation_matrix.determinant() < 0.0 {
        let mut adjusted_v_t = v_t;
        adjusted_v_t[(1, 0)] *= -1.0;
        adjusted_v_t[(1, 1)] *= -1.0;
        rotation_matrix = adjusted_v_t.transpose() * u.transpose();
    }
    let rotation_angle = rotation_matrix[(1, 0)].atan2(rotation_matrix[(0, 0)]);
    let translation = reference_centroid - Rotation2::new(rotation_angle) * measured_centroid;

    Some(nalgebra::Isometry2::new(translation, rotation_angle))
}

fn locality_cost(
    correction: nalgebra::Isometry2<f32>,
    locality_parameters: &LocalityParameters,
) -> f32 {
    let translation = correction.translation.vector / locality_parameters.translation_scale;
    let rotation =
        normalized_angle(correction.rotation.angle()) / locality_parameters.rotation_scale;

    translation.norm_squared() + rotation.powi(2)
}

fn locality_translation_gradient(
    correction: nalgebra::Isometry2<f32>,
    locality_parameters: &LocalityParameters,
) -> Vector2<f32> {
    let denominator = locality_parameters.translation_scale.powi(2);
    2.0 * locality_parameters.weight * correction.translation.vector / denominator
}

fn locality_rotation_gradient(
    correction: nalgebra::Isometry2<f32>,
    locality_parameters: &LocalityParameters,
) -> f32 {
    let denominator = locality_parameters.rotation_scale.powi(2);
    2.0 * locality_parameters.weight * normalized_angle(correction.rotation.angle()) / denominator
}

fn normalized_angle(angle: f32) -> f32 {
    UnitComplex::new(angle).angle()
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
        f32::consts::{FRAC_PI_2, PI},
        time::{Duration, SystemTime},
    };

    use approx::assert_relative_eq;
    use linear_algebra::Point2;
    use nalgebra::{matrix, vector};
    use types::field_marks::Direction;

    use super::*;

    fn test_matching_parameters() -> MatchingParameters {
        MatchingParameters {
            ambiguity_margin_threshold: 0.2,
            gradient_convergence_threshold: 0.0001,
            gradient_descent_step_size: 0.1,
            line_length_acceptance_factor: 1.5,
            locality: LocalityParameters {
                translation_scale: 1.0,
                rotation_scale: 0.5,
                weight: 0.5,
            },
            maximum_amount_of_gradient_descent_iterations: 8,
            minimum_fit_error: 0.001,
            minimum_inlier_count: 1,
            minimum_measurement_confidence: 0.05,
            ransac_inlier_threshold: 0.2,
            ransac_iterations: 8,
        }
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
        let result = get_fitted_field_mark_correspondence(
            &[],
            &[FieldMark::Line {
                line: LineSegment(point![0.0, 0.0], point![1.0, 0.0]),
                direction: Direction::PositiveX,
            }],
            &test_matching_parameters(),
        );

        assert!(result.is_none());
    }

    #[test]
    fn zero_length_field_marks_are_ignored() {
        let correspondences = get_line_candidates(
            0,
            LineSegment(point![0.0, 0.0], point![1.0, 0.0]),
            nalgebra::Isometry2::identity(),
            &[FieldMark::Circle {
                center: Point2::origin(),
                radius: 0.0,
            }],
            1.5,
            &test_matching_parameters().locality,
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
    fn false_positive_line_remains_unmatched() {
        let matching_parameters = test_matching_parameters();
        let field_marks = [FieldMark::Line {
            line: LineSegment(point![0.0, 0.0], point![2.0, 0.0]),
            direction: Direction::PositiveX,
        }];
        let measured_lines = [
            LineSegment(point![0.0, 0.0], point![1.0, 0.0]),
            LineSegment(point![4.0, 4.0], point![5.0, 4.0]),
        ];

        let result = get_fitted_field_mark_correspondence(
            &measured_lines,
            &field_marks,
            &matching_parameters,
        )
        .unwrap();

        assert_eq!(result.matched_lines, 1);
        assert_eq!(result.unmatched_lines, 1);
    }

    #[test]
    fn fragmented_lines_can_match_the_same_field_mark() {
        let matching_parameters = test_matching_parameters();
        let field_mark = FieldMark::Line {
            line: LineSegment(point![0.0, 0.0], point![2.0, 0.0]),
            direction: Direction::PositiveX,
        };
        let measured_lines = [
            LineSegment(point![0.0, 0.0], point![0.8, 0.0]),
            LineSegment(point![1.2, 0.0], point![2.0, 0.0]),
        ];

        let result = get_fitted_field_mark_correspondence(
            &measured_lines,
            &[field_mark],
            &matching_parameters,
        )
        .unwrap();

        assert_eq!(result.matched_lines, 2);
        assert_eq!(result.field_mark_correspondences.len(), 2);
    }

    #[test]
    fn candidate_summaries_are_capped_at_three_alternatives() {
        let matching_parameters = test_matching_parameters();
        let evaluation = summarize_correction(
            &[LineSegment(point![0.0, 0.0], point![0.8, 0.0])],
            &[
                FieldMark::Line {
                    line: LineSegment(point![0.0, 0.0], point![2.0, 0.0]),
                    direction: Direction::PositiveX,
                },
                FieldMark::Line {
                    line: LineSegment(point![0.0, 0.2], point![2.0, 0.2]),
                    direction: Direction::PositiveX,
                },
                FieldMark::Line {
                    line: LineSegment(point![0.0, -0.2], point![2.0, -0.2]),
                    direction: Direction::PositiveX,
                },
                FieldMark::Line {
                    line: LineSegment(point![0.0, 0.4], point![2.0, 0.4]),
                    direction: Direction::PositiveX,
                },
            ],
            nalgebra::Isometry2::identity(),
            &matching_parameters,
        );

        assert_eq!(evaluation.candidate_summaries.len(), 1);
        assert_eq!(evaluation.candidate_summaries[0].candidates.len(), 3);
    }

    #[test]
    fn locality_confidence_decreases_for_larger_corrections() {
        let locality = test_matching_parameters().locality;
        let near_correction = nalgebra::Isometry2::new(vector![0.1, 0.0], 0.0);
        let far_correction = nalgebra::Isometry2::new(vector![1.0, 0.0], 0.0);
        let near_confidence = 1.0 / (1.0 + locality_cost(near_correction, &locality).sqrt());
        let far_confidence = 1.0 / (1.0 + locality_cost(far_correction, &locality).sqrt());

        assert!(near_confidence > far_confidence);
    }

    #[test]
    fn estimate_correction_recovers_translation_and_rotation() {
        let measured = [
            CorrespondencePoints {
                measured: point![0.0, 0.0],
                reference: point![1.0, 2.0],
            },
            CorrespondencePoints {
                measured: point![1.0, 0.0],
                reference: point![1.0, 3.0],
            },
        ];

        let correction = estimate_correction_from_correspondence_points(&measured).unwrap();

        assert_relative_eq!(correction.translation.vector.x, 1.0, epsilon = 0.0001);
        assert_relative_eq!(correction.translation.vector.y, 2.0, epsilon = 0.0001);
        assert_relative_eq!(correction.rotation.angle(), FRAC_PI_2, epsilon = 0.0001);
    }

    #[test]
    fn single_field_line_leaves_tangent_direction_weakly_observed() {
        let field_mark_correspondence = FieldMarkCorrespondence {
            measured_line_index: 0,
            measured_line_in_field: LineSegment(point![0.0, 0.0], point![2.0, 0.0]),
            field_mark: FieldMark::Line {
                line: LineSegment(point![0.0, 1.0], point![2.0, 1.0]),
                direction: Direction::PositiveX,
            },
            correspondence_points: (
                CorrespondencePoints {
                    measured: point![0.0, 0.0],
                    reference: point![0.0, 1.0],
                },
                CorrespondencePoints {
                    measured: point![2.0, 0.0],
                    reference: point![2.0, 1.0],
                },
            ),
        };

        let covariance = measurement_covariance_from_correspondences(
            Matrix3::from_diagonal(&vector![600.0, 600.0, 100.0]),
            &[field_mark_correspondence],
            nalgebra::Isometry2::identity(),
            1.0,
        );

        assert!(covariance[(0, 0)] > covariance[(1, 1)]);
        assert!(covariance[(0, 0)] > 1000.0);
    }
}
