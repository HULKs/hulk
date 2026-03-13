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
use nalgebra::{Matrix2, Matrix3, Rotation2, Vector2, Vector3, matrix, vector};
use ordered_float::NotNan;
use projection::{Projection, camera_matrix::CameraMatrix};
use serde::{Deserialize, Serialize};

use context_attribute::context;
use coordinate_systems::{Field, Ground};
use filtering::pose_filter::PoseFilter;
use framework::{AdditionalOutput, HistoricInput, MainOutput, PerceptionInput};
use hsl_network_messages::{GamePhase, Penalty, PlayerNumber, SubState, Team};
use types::{
    cycle_time::CycleTime,
    field_dimensions::{FieldDimensions, Half, Side as FieldSide},
    field_marks::{CorrespondencePoints, Direction, FieldMark, field_marks_from_field_dimensions},
    filtered_game_controller_state::FilteredGameControllerState,
    initial_pose::InitialPose,
    line_data::LineData,
    localization::{
        GoalPostPairAssociationDebug, LineAssociationDebug, LocalizationDebugFrame,
        LocalizationDebugHypothesis, PointAssociationDebug, ScoredPose,
    },
    multivariate_normal_distribution::MultivariateNormalDistribution,
    object_detection::{Detection, NaoLabelPartyObjectDetectionLabel},
    players::Players,
    primary_state::PrimaryState,
    support_foot::Side,
};

type FitErrorsPerGradientStep = Vec<f32>;
type FitErrorsPerOuterIteration = Vec<FitErrorsPerGradientStep>;

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
    last_detected_objects_time: SystemTime,
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
    camera_matrix: HistoricInput<Option<CameraMatrix>, "camera_matrix?">,

    odometer: PerceptionInput<Odometer, "Odometry", "odometer">,
    fall_down_state: PerceptionInput<Option<FallDownState>, "FallDownState", "fall_down_state?">,
    imu_state: PerceptionInput<ImuState, "Motion", "imu_state">,
    line_data: PerceptionInput<Option<LineData>, "Vision", "line_data?">,
    detected_objects: PerceptionInput<
        Vec<Detection<NaoLabelPartyObjectDetectionLabel>>,
        "ObjectDetection",
        "detected_objects",
    >,

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
    penalty_spot_matching_distance: Parameter<f32, "localization.penalty_spot_matching_distance">,
    penalty_spot_measurement_noise:
        Parameter<Vector2<f32>, "localization.penalty_spot_measurement_noise">,
    player_number: Parameter<PlayerNumber, "player_number">,
    penalized_distance: Parameter<f32, "localization.penalized_distance">,
    penalized_hypothesis_covariance:
        Parameter<Matrix3<f32>, "localization.penalized_hypothesis_covariance">,
    goal_post_pair_matching_distance:
        Parameter<f32, "localization.goal_post_pair_matching_distance">,
    goal_post_pair_measurement_noise:
        Parameter<Matrix3<f32>, "localization.goal_post_pair_measurement_noise">,
    score_per_good_match: Parameter<f32, "localization.score_per_good_match">,
    single_goal_post_matching_distance:
        Parameter<f32, "localization.single_goal_post_matching_distance">,
    single_goal_post_measurement_noise:
        Parameter<Vector2<f32>, "localization.single_goal_post_measurement_noise">,
    tentative_penalized_duration: Parameter<Duration, "localization.tentative_penalized_duration">,
    use_detected_field_mark_measurements:
        Parameter<bool, "localization.use_detected_field_mark_measurements">,
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

struct MeasurementNoise {
    line: Matrix2<f32>,
    circle: Matrix2<f32>,
    penalty_spot: Matrix2<f32>,
    single_goal_post: Matrix2<f32>,
    goal_post_pair: Matrix3<f32>,
}

#[derive(Default)]
struct HypothesisDebugState {
    line_associations: Vec<LineAssociationDebug>,
    unmatched_lines_in_field: Vec<LineSegment<Field>>,
    penalty_spot_associations: Vec<PointAssociationDebug>,
    unmatched_penalty_spots_in_field: Vec<Point2<Field>>,
    single_goal_post_associations: Vec<PointAssociationDebug>,
    unmatched_goal_posts_in_field: Vec<Point2<Field>>,
    goal_post_pair_association: Option<GoalPostPairAssociationDebug>,
}

#[derive(Default)]
struct DetectedFieldMarks {
    penalty_spots_in_ground: Vec<Point2<Ground>>,
    goal_posts_in_ground: Vec<Point2<Ground>>,
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
    detected_field_marks: Option<DetectedFieldMarks>,
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
            last_line_data_time: SystemTime::UNIX_EPOCH,
            last_detected_objects_time: SystemTime::UNIX_EPOCH,
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

        let current_odometer = Self::latest_odometer(context);
        let current_odometry_to_last_odometry = match (self.last_odometer, current_odometer) {
            (Some(last), Some(latest)) => odometry_delta(last, latest),
            _ => Default::default(),
        };
        self.last_odometer = current_odometer;

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

        let detected_field_marks = context
            .detected_objects
            .persistent
            .iter()
            .chain(&context.detected_objects.temporary)
            .filter(|(time, _)| **time > self.last_detected_objects_time)
            .flat_map(|(time, detections)| Some((*time, (*detections.iter().last()?).clone())))
            .last()
            .map(|(time, detections)| {
                self.last_detected_objects_time = time;
                extract_detected_field_marks(context, time, &detections)
            })
            .flatten();

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
            detected_field_marks,
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
        let measurement_noise = self.measurement_noise(context, inputs);
        self.predict_hypotheses(context, inputs.current_odometry_to_last_odometry)?;
        let debug_hypotheses = self.apply_measurements(inputs, context, &measurement_noise)?;
        self.finalize_hypotheses(context, inputs, debug_hypotheses)
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
            penalty_spot: Matrix2::from_diagonal(&context.penalty_spot_measurement_noise),
            single_goal_post: Matrix2::from_diagonal(&context.single_goal_post_measurement_noise),
            goal_post_pair: *context.goal_post_pair_measurement_noise,
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
        measurement_noise: &MeasurementNoise,
    ) -> Result<Vec<LocalizationDebugHypothesis>> {
        let mut debug_hypotheses = Vec::with_capacity(self.hypotheses.len());
        let field_marks = self.field_marks.as_slice();

        for scored_state in &mut self.hypotheses {
            let mut debug_state = HypothesisDebugState::default();

            if *context.use_line_measurements && inputs.line_measurements_allowed {
                if let Some(line_data) = inputs.line_data.as_ref() {
                    Self::apply_line_measurements_for_hypothesis(
                        field_marks,
                        scored_state,
                        line_data,
                        context,
                        measurement_noise,
                        &mut debug_state,
                    )?;
                }
            }

            if *context.use_detected_field_mark_measurements && inputs.line_measurements_allowed {
                if let Some(detected_field_marks) = inputs.detected_field_marks.as_ref() {
                    Self::apply_detected_field_mark_measurements_for_hypothesis(
                        scored_state,
                        detected_field_marks,
                        context,
                        measurement_noise,
                        &mut debug_state,
                    )?;
                }
            }

            scored_state.score += *context.hypothesis_score_base_increase;
            debug_hypotheses.push(LocalizationDebugHypothesis {
                ground_to_field: scored_state.state.as_isometry().framed_transform(),
                score: scored_state.score,
                covariance: scored_state.state.covariance,
                line_associations: debug_state.line_associations,
                unmatched_lines_in_field: debug_state.unmatched_lines_in_field,
                penalty_spot_associations: debug_state.penalty_spot_associations,
                unmatched_penalty_spots_in_field: debug_state.unmatched_penalty_spots_in_field,
                single_goal_post_associations: debug_state.single_goal_post_associations,
                unmatched_goal_posts_in_field: debug_state.unmatched_goal_posts_in_field,
                goal_post_pair_association: debug_state.goal_post_pair_association,
            });
        }

        Ok(debug_hypotheses)
    }

    fn apply_line_measurements_for_hypothesis(
        field_marks: &[FieldMark],
        scored_state: &mut ScoredPose,
        line_data: &LineData,
        context: &CycleContext,
        measurement_noise: &MeasurementNoise,
        debug_state: &mut HypothesisDebugState,
    ) -> Result<()> {
        let ground_to_field: Isometry2<Ground, Field> =
            scored_state.state.as_isometry().framed_transform();
        let measured_lines_in_field: Vec<_> = line_data
            .lines
            .iter()
            .map(|&measured_line_in_ground| ground_to_field * measured_line_in_ground)
            .collect();

        if measured_lines_in_field.is_empty() {
            return Ok(());
        }

        let (field_mark_correspondences, fit_error, _fit_errors) =
            get_fitted_field_mark_correspondence(
                &measured_lines_in_field,
                field_marks,
                *context.gradient_convergence_threshold,
                *context.gradient_descent_step_size,
                *context.line_length_acceptance_factor,
                *context.maximum_amount_of_gradient_descent_iterations,
                *context.maximum_amount_of_outer_iterations,
                false,
            );

        let matched_line_indices = field_mark_correspondences
            .iter()
            .map(|correspondence| correspondence.measured_line_index)
            .collect::<std::collections::HashSet<_>>();
        debug_state.unmatched_lines_in_field.extend(
            measured_lines_in_field
                .iter()
                .enumerate()
                .filter(|(index, _)| !matched_line_indices.contains(index))
                .map(|(_, &line)| line),
        );

        if field_mark_correspondences.is_empty() {
            return Ok(());
        }

        let clamped_fit_error = fit_error.max(*context.minimum_fit_error);
        let number_of_measurements_weight = 1.0 / field_mark_correspondences.len() as f32;

        for field_mark_correspondence in field_mark_correspondences {
            debug_state.line_associations.push(LineAssociationDebug {
                measured_line: field_mark_correspondence.measured_line_in_field,
                matched_field_mark: field_mark_correspondence.field_mark,
                correspondence_points: field_mark_correspondence.correspondence_points,
                fit_error: field_mark_correspondence.fit_error_sum(),
            });

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
                            Direction::PositiveX => vector![state.y, state.z],
                            Direction::PositiveY => vector![state.x, state.z],
                        },
                    )
                    .context("failed to update pose filter with line correspondence")?,
                FieldMark::Circle { .. } => scored_state
                    .state
                    .update_with_2d_translation(
                        update,
                        measurement_noise.circle * uncertainty_weight,
                        |state| vector![state.x, state.y],
                    )
                    .context("failed to update pose filter with circle correspondence")?,
            }

            if field_mark_correspondence.fit_error_sum() < *context.good_matching_threshold {
                scored_state.score += *context.score_per_good_match;
            }
        }

        Ok(())
    }

    fn apply_detected_field_mark_measurements_for_hypothesis(
        scored_state: &mut ScoredPose,
        detected_field_marks: &DetectedFieldMarks,
        context: &CycleContext,
        measurement_noise: &MeasurementNoise,
        debug_state: &mut HypothesisDebugState,
    ) -> Result<()> {
        Self::apply_penalty_spot_measurements_for_hypothesis(
            scored_state,
            &detected_field_marks.penalty_spots_in_ground,
            context,
            measurement_noise,
            debug_state,
        )?;
        Self::apply_goal_post_measurements_for_hypothesis(
            scored_state,
            &detected_field_marks.goal_posts_in_ground,
            context,
            measurement_noise,
            debug_state,
        )?;
        Ok(())
    }

    fn apply_penalty_spot_measurements_for_hypothesis(
        scored_state: &mut ScoredPose,
        measured_penalty_spots_in_ground: &[Point2<Ground>],
        context: &CycleContext,
        measurement_noise: &MeasurementNoise,
        debug_state: &mut HypothesisDebugState,
    ) -> Result<()> {
        let reference_points = [
            context.field_dimensions.penalty_spot(Half::Own),
            context.field_dimensions.penalty_spot(Half::Opponent),
        ];

        for &measured_penalty_spot_in_ground in measured_penalty_spots_in_ground {
            let ground_to_field: Isometry2<Ground, Field> =
                scored_state.state.as_isometry().framed_transform();
            let measured_penalty_spot_in_field = ground_to_field * measured_penalty_spot_in_ground;
            let (matched_reference_point, association_distance) =
                nearest_reference_point(measured_penalty_spot_in_field, &reference_points);
            let accepted = association_distance <= *context.penalty_spot_matching_distance;

            debug_state
                .penalty_spot_associations
                .push(PointAssociationDebug {
                    measured_point_in_field: measured_penalty_spot_in_field,
                    matched_reference_point: Some(matched_reference_point),
                    association_distance: Some(association_distance),
                    matching_distance: *context.penalty_spot_matching_distance,
                    accepted,
                });

            if !accepted {
                debug_state
                    .unmatched_penalty_spots_in_field
                    .push(measured_penalty_spot_in_field);
                continue;
            }

            scored_state
                .state
                .update_with_2d_translation(
                    measured_point_to_vector(matched_reference_point),
                    measurement_noise.penalty_spot,
                    |state| {
                        let ground_to_field =
                            nalgebra::Isometry2::new(vector![state.x, state.y], state.z)
                                .framed_transform();
                        measured_point_to_vector(ground_to_field * measured_penalty_spot_in_ground)
                    },
                )
                .context("failed to update pose filter with penalty spot measurement")?;
            scored_state.score += *context.score_per_good_match;
        }

        Ok(())
    }

    fn apply_goal_post_measurements_for_hypothesis(
        scored_state: &mut ScoredPose,
        measured_goal_posts_in_ground: &[Point2<Ground>],
        context: &CycleContext,
        measurement_noise: &MeasurementNoise,
        debug_state: &mut HypothesisDebugState,
    ) -> Result<()> {
        if measured_goal_posts_in_ground.len() >= 2 {
            return Self::apply_goal_post_pair_measurement_for_hypothesis(
                scored_state,
                measured_goal_posts_in_ground,
                context,
                measurement_noise,
                debug_state,
            );
        }

        let reference_points = goal_post_reference_points(context.field_dimensions);
        for &measured_goal_post_in_ground in measured_goal_posts_in_ground {
            let ground_to_field: Isometry2<Ground, Field> =
                scored_state.state.as_isometry().framed_transform();
            let measured_goal_post_in_field = ground_to_field * measured_goal_post_in_ground;
            let (matched_reference_point, association_distance) =
                nearest_reference_point(measured_goal_post_in_field, &reference_points);
            let accepted = association_distance <= *context.single_goal_post_matching_distance;

            debug_state
                .single_goal_post_associations
                .push(PointAssociationDebug {
                    measured_point_in_field: measured_goal_post_in_field,
                    matched_reference_point: Some(matched_reference_point),
                    association_distance: Some(association_distance),
                    matching_distance: *context.single_goal_post_matching_distance,
                    accepted,
                });

            if !accepted {
                debug_state
                    .unmatched_goal_posts_in_field
                    .push(measured_goal_post_in_field);
                continue;
            }

            scored_state
                .state
                .update_with_2d_translation(
                    measured_point_to_vector(matched_reference_point),
                    measurement_noise.single_goal_post,
                    |state| {
                        let ground_to_field =
                            nalgebra::Isometry2::new(vector![state.x, state.y], state.z)
                                .framed_transform();
                        measured_point_to_vector(ground_to_field * measured_goal_post_in_ground)
                    },
                )
                .context("failed to update pose filter with single goal post measurement")?;
            scored_state.score += *context.score_per_good_match;
        }

        Ok(())
    }

    fn apply_goal_post_pair_measurement_for_hypothesis(
        scored_state: &mut ScoredPose,
        measured_goal_posts_in_ground: &[Point2<Ground>],
        context: &CycleContext,
        measurement_noise: &MeasurementNoise,
        debug_state: &mut HypothesisDebugState,
    ) -> Result<()> {
        let ground_to_field: Isometry2<Ground, Field> =
            scored_state.state.as_isometry().framed_transform();
        let Some(best_pair_match) = best_goal_post_pair_match(
            ground_to_field,
            measured_goal_posts_in_ground,
            context.field_dimensions,
        ) else {
            return Ok(());
        };
        let accepted =
            best_pair_match.average_distance <= *context.goal_post_pair_matching_distance;

        debug_state.goal_post_pair_association = Some(GoalPostPairAssociationDebug {
            measured_posts_in_field: best_pair_match.measured_posts_in_field,
            matched_reference_posts: best_pair_match.reference_posts,
            pair_fit_error: best_pair_match.average_distance,
            matching_distance: *context.goal_post_pair_matching_distance,
            accepted,
            resulting_ground_to_field: accepted
                .then_some(best_pair_match.estimated_ground_to_field),
        });

        if !accepted {
            debug_state.unmatched_goal_posts_in_field.extend(
                measured_goal_posts_in_ground
                    .iter()
                    .map(|&goal_post| ground_to_field * goal_post),
            );
            return Ok(());
        }

        let measurement = vector![
            best_pair_match.estimated_ground_to_field.translation().x(),
            best_pair_match.estimated_ground_to_field.translation().y(),
            best_pair_match
                .estimated_ground_to_field
                .orientation()
                .angle(),
        ];
        scored_state
            .state
            .update_with_3d_pose(measurement, measurement_noise.goal_post_pair, |state| state)
            .context("failed to update pose filter with goal post pair measurement")?;
        scored_state.score += *context.score_per_good_match;

        Ok(())
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

        context
            .debug_frame
            .fill_if_subscribed(|| LocalizationDebugFrame {
                cycle_start_time: unix_duration(inputs.cycle_start_time),
                gyro_movement: inputs.gyro_movement,
                best_hypothesis_index: debug_hypotheses
                    .iter()
                    .enumerate()
                    .max_by_key(|(_, hypothesis)| NotNan::new(hypothesis.score).unwrap())
                    .map(|(index, _)| index),
                hypotheses: debug_hypotheses,
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

struct GoalPostPairMatch {
    measured_posts_in_field: (Point2<Field>, Point2<Field>),
    reference_posts: (Point2<Field>, Point2<Field>),
    average_distance: f32,
    estimated_ground_to_field: Isometry2<Ground, Field>,
}

fn extract_detected_field_marks(
    context: &CycleContext,
    detection_time: SystemTime,
    detections: &[Detection<NaoLabelPartyObjectDetectionLabel>],
) -> Option<DetectedFieldMarks> {
    let camera_matrix = context.camera_matrix.get_nearest(&detection_time)?;
    let mut detected_field_marks = DetectedFieldMarks::default();

    for detection in detections {
        let measured_point_in_ground = match detection.label {
            NaoLabelPartyObjectDetectionLabel::PenaltySpot => camera_matrix
                .pixel_to_ground(bounding_box_center(detection))
                .ok(),
            NaoLabelPartyObjectDetectionLabel::GoalPost => camera_matrix
                .pixel_to_ground(bounding_box_bottom_center(detection))
                .ok(),
            _ => None,
        };

        let Some(measured_point_in_ground) = measured_point_in_ground else {
            continue;
        };
        match detection.label {
            NaoLabelPartyObjectDetectionLabel::PenaltySpot => {
                detected_field_marks
                    .penalty_spots_in_ground
                    .push(measured_point_in_ground);
            }
            NaoLabelPartyObjectDetectionLabel::GoalPost => {
                detected_field_marks
                    .goal_posts_in_ground
                    .push(measured_point_in_ground);
            }
            _ => {}
        }
    }

    (!detected_field_marks.penalty_spots_in_ground.is_empty()
        || !detected_field_marks.goal_posts_in_ground.is_empty())
    .then_some(detected_field_marks)
}

fn bounding_box_center(
    detection: &Detection<NaoLabelPartyObjectDetectionLabel>,
) -> linear_algebra::Point2<coordinate_systems::Pixel> {
    let area = detection.bounding_box.area;
    point![
        area.min.x() + (area.max.x() - area.min.x()) / 2.0,
        area.min.y() + (area.max.y() - area.min.y()) / 2.0
    ]
}

fn bounding_box_bottom_center(
    detection: &Detection<NaoLabelPartyObjectDetectionLabel>,
) -> linear_algebra::Point2<coordinate_systems::Pixel> {
    let area = detection.bounding_box.area;
    point![
        area.min.x() + (area.max.x() - area.min.x()) / 2.0,
        area.max.y()
    ]
}

fn goal_post_reference_points(field_dimensions: &FieldDimensions) -> [Point2<Field>; 4] {
    [
        field_dimensions.goal_post(Half::Own, FieldSide::Left),
        field_dimensions.goal_post(Half::Own, FieldSide::Right),
        field_dimensions.goal_post(Half::Opponent, FieldSide::Left),
        field_dimensions.goal_post(Half::Opponent, FieldSide::Right),
    ]
}

fn nearest_reference_point(
    measured_point_in_field: Point2<Field>,
    reference_points: &[Point2<Field>],
) -> (Point2<Field>, f32) {
    reference_points
        .iter()
        .copied()
        .map(|reference_point| {
            (
                reference_point,
                distance(measured_point_in_field, reference_point),
            )
        })
        .min_by_key(|(_, distance)| NotNan::new(*distance).unwrap())
        .unwrap()
}

fn measured_point_to_vector(point: Point2<Field>) -> Vector2<f32> {
    vector![point.x(), point.y()]
}

fn best_goal_post_pair_match(
    ground_to_field: Isometry2<Ground, Field>,
    measured_goal_posts_in_ground: &[Point2<Ground>],
    field_dimensions: &FieldDimensions,
) -> Option<GoalPostPairMatch> {
    let goal_pairs = [
        (
            field_dimensions.goal_post(Half::Own, FieldSide::Left),
            field_dimensions.goal_post(Half::Own, FieldSide::Right),
        ),
        (
            field_dimensions.goal_post(Half::Opponent, FieldSide::Left),
            field_dimensions.goal_post(Half::Opponent, FieldSide::Right),
        ),
    ];

    measured_goal_posts_in_ground
        .iter()
        .enumerate()
        .flat_map(|(first_index, &first_post)| {
            measured_goal_posts_in_ground
                .iter()
                .enumerate()
                .skip(first_index + 1)
                .map(move |(_, &second_post)| (first_post, second_post))
        })
        .flat_map(|(first_post, second_post)| {
            goal_pairs.into_iter().flat_map(move |reference_posts| {
                [
                    ((first_post, second_post), reference_posts),
                    (
                        (second_post, first_post),
                        (reference_posts.0, reference_posts.1),
                    ),
                ]
            })
        })
        .filter_map(|(measured_posts_in_ground, reference_posts)| {
            let measured_posts_in_field = (
                ground_to_field * measured_posts_in_ground.0,
                ground_to_field * measured_posts_in_ground.1,
            );
            let average_distance = 0.5
                * (distance(measured_posts_in_field.0, reference_posts.0)
                    + distance(measured_posts_in_field.1, reference_posts.1));
            let estimated_ground_to_field = estimate_ground_to_field_from_point_pairs(
                measured_posts_in_ground,
                reference_posts,
            )?;
            Some(GoalPostPairMatch {
                measured_posts_in_field,
                reference_posts,
                average_distance,
                estimated_ground_to_field,
            })
        })
        .min_by_key(|pair_match| NotNan::new(pair_match.average_distance).unwrap())
}

fn estimate_ground_to_field_from_point_pairs(
    measured_points_in_ground: (Point2<Ground>, Point2<Ground>),
    reference_points_in_field: (Point2<Field>, Point2<Field>),
) -> Option<Isometry2<Ground, Field>> {
    let measured_point_0 = measured_points_in_ground.0.coords().inner;
    let measured_point_1 = measured_points_in_ground.1.coords().inner;
    let reference_point_0 = reference_points_in_field.0.coords().inner;
    let reference_point_1 = reference_points_in_field.1.coords().inner;
    let measured_centroid = (measured_point_0 + measured_point_1) / 2.0;
    let reference_centroid = (reference_point_0 + reference_point_1) / 2.0;
    let covariance = (measured_point_0 - measured_centroid)
        * (reference_point_0 - reference_centroid).transpose()
        + (measured_point_1 - measured_centroid)
            * (reference_point_1 - reference_centroid).transpose();
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

    Some(nalgebra::Isometry2::new(translation, rotation_angle).framed_transform::<Ground, Field>())
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
    measured_line_index: usize,
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
        .enumerate()
        .filter_map(|(measured_line_index, &measured_line_in_field)| {
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
                measured_line_index,
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
    vec![Pose2::new(
        point![
            -field_dimensions.length * 0.5 + field_dimensions.penalty_marker_distance,
            field_dimensions.width * 0.5 + penalized_distance
        ],
        -FRAC_PI_2,
    )]
}

#[cfg(test)]
mod tests {
    use std::{
        f32::consts::{FRAC_PI_4, PI},
        time::{Duration, SystemTime},
    };

    use approx::assert_relative_eq;
    use linear_algebra::Point2;
    use nalgebra::{matrix, vector};

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
            measured_line_index: 0,
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
            measured_line_index: 0,
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
            measured_line_index: 0,
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
            measured_line_index: 0,
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
            measured_line_index: 0,
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
            measured_line_index: 0,
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

    #[test]
    fn nearest_penalty_spot_is_selected() {
        let field_dimensions = FieldDimensions::SPL_2025;
        let reference_points = [
            field_dimensions.penalty_spot(Half::Own),
            field_dimensions.penalty_spot(Half::Opponent),
        ];

        let (reference_point, distance) =
            nearest_reference_point(point![3.1, 0.1], &reference_points);

        assert_relative_eq!(
            reference_point,
            field_dimensions.penalty_spot(Half::Opponent)
        );
        assert!(distance < 0.3);
    }

    #[test]
    fn estimated_ground_to_field_from_goal_post_pair_recovers_pose() {
        let measured_points_in_ground = (point![0.0, -0.75], point![0.0, 0.75]);
        let reference_points_in_field = (point![4.0, -1.0], point![4.0, 1.0]);

        let estimated_ground_to_field = estimate_ground_to_field_from_point_pairs(
            measured_points_in_ground,
            reference_points_in_field,
        )
        .unwrap();

        assert_relative_eq!(
            estimated_ground_to_field.translation().x(),
            4.0,
            epsilon = 1.0e-4
        );
        assert_relative_eq!(
            estimated_ground_to_field.translation().y(),
            0.0,
            epsilon = 1.0e-4
        );
        assert_relative_eq!(
            estimated_ground_to_field.orientation().angle(),
            0.0,
            epsilon = 1.0e-4
        );
    }
}
