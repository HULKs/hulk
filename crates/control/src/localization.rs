use color_eyre::Result;
use context_attribute::context;
use filtering::ScoredPoseFilter;
use framework::{AdditionalOutput, HistoricInput, MainOutput, PerceptionInput};
use nalgebra::{Isometry2, Matrix3, Vector2, Vector3};
use spl_network_messages::PlayerNumber;
use types::{
    FieldDimensions, GameControllerState, InitialPose, Line2, LineData, LocalizationUpdate,
    Players, PrimaryState,
};

pub struct Localization {}

#[context]
pub struct CreationContext {
    pub circle_measurement_noise:
        Parameter<Vector2<f32>, "control/localization/circle_measurement_noise">,
    pub field_dimensions: Parameter<FieldDimensions, "field_dimensions">,
    pub good_matching_threshold: Parameter<f32, "control/localization/good_matching_threshold">,
    pub gradient_convergence_threshold:
        Parameter<f32, "control/localization/gradient_convergence_threshold">,
    pub gradient_descent_step_size:
        Parameter<f32, "control/localization/gradient_descent_step_size">,
    pub hypothesis_prediction_score_reduction_factor:
        Parameter<f32, "control/localization/hypothesis_prediction_score_reduction_factor">,
    pub hypothesis_retain_factor: Parameter<f32, "control/localization/hypothesis_retain_factor">,
    pub hypothesis_score_base_increase:
        Parameter<f32, "control/localization/hypothesis_score_base_increase">,
    pub initial_hypothesis_covariance:
        Parameter<Matrix3<f32>, "control/localization/initial_hypothesis_covariance">,
    pub initial_hypothesis_score: Parameter<f32, "control/localization/initial_hypothesis_score">,
    pub initial_poses: Parameter<Players<InitialPose>, "control/localization/initial_poses">,
    pub line_length_acceptance_factor:
        Parameter<f32, "control/localization/line_length_acceptance_factor">,
    pub line_measurement_noise:
        Parameter<Vector2<f32>, "control/localization/line_measurement_noise">,
    pub maximum_amount_of_gradient_descent_iterations:
        Parameter<usize, "control/localization/maximum_amount_of_gradient_descent_iterations">,
    pub maximum_amount_of_outer_iterations:
        Parameter<usize, "control/localization/maximum_amount_of_outer_iterations">,
    pub minimum_fit_error: Parameter<f32, "control/localization/minimum_fit_error">,
    pub odometry_noise: Parameter<Vector3<f32>, "control/localization/odometry_noise">,
    pub player_number: Parameter<PlayerNumber, "player_number">,
    pub score_per_good_match: Parameter<f32, "control/localization/score_per_good_match">,
    pub use_line_measurements: Parameter<bool, "control/localization/use_line_measurements">,

    pub robot_to_field: PersistentState<Isometry2<f32>, "robot_to_field">,
}

#[context]
pub struct CycleContext {
    pub correspondence_lines: AdditionalOutput<Vec<Line2>, "localization/correspondence_lines">,
    pub fit_errors: AdditionalOutput<Vec<Vec<Vec<Vec<f32>>>>, "localization/fit_errors">,
    pub measured_lines_in_field:
        AdditionalOutput<Vec<Line2>, "localization/measured_lines_in_field">,
    pub pose_hypotheses: AdditionalOutput<Vec<ScoredPoseFilter>, "localization/pose_hypotheses">,
    pub updates: AdditionalOutput<Vec<Vec<LocalizationUpdate>>, "localization/updates">,

    pub current_odometry_to_last_odometry:
        HistoricInput<Option<Isometry2<f32>>, "current_odometry_to_last_odometry?">,

    pub game_controller_state: RequiredInput<Option<GameControllerState>, "game_controller_state?">,
    pub has_ground_contact: Input<bool, "has_ground_contact">,
    pub primary_state: RequiredInput<Option<PrimaryState>, "primary_state?">,

    pub circle_measurement_noise:
        Parameter<Vector2<f32>, "control/localization/circle_measurement_noise">,
    pub field_dimensions: Parameter<FieldDimensions, "field_dimensions">,
    pub good_matching_threshold: Parameter<f32, "control/localization/good_matching_threshold">,
    pub gradient_convergence_threshold:
        Parameter<f32, "control/localization/gradient_convergence_threshold">,
    pub gradient_descent_step_size:
        Parameter<f32, "control/localization/gradient_descent_step_size">,
    pub hypothesis_prediction_score_reduction_factor:
        Parameter<f32, "control/localization/hypothesis_prediction_score_reduction_factor">,
    pub hypothesis_retain_factor: Parameter<f32, "control/localization/hypothesis_retain_factor">,
    pub hypothesis_score_base_increase:
        Parameter<f32, "control/localization/hypothesis_score_base_increase">,
    pub initial_hypothesis_covariance:
        Parameter<Matrix3<f32>, "control/localization/initial_hypothesis_covariance">,
    pub initial_hypothesis_score: Parameter<f32, "control/localization/initial_hypothesis_score">,
    pub initial_poses: Parameter<Players<InitialPose>, "control/localization/initial_poses">,
    pub line_length_acceptance_factor:
        Parameter<f32, "control/localization/line_length_acceptance_factor">,
    pub line_measurement_noise:
        Parameter<Vector2<f32>, "control/localization/line_measurement_noise">,
    pub maximum_amount_of_gradient_descent_iterations:
        Parameter<usize, "control/localization/maximum_amount_of_gradient_descent_iterations">,
    pub maximum_amount_of_outer_iterations:
        Parameter<usize, "control/localization/maximum_amount_of_outer_iterations">,
    pub minimum_fit_error: Parameter<f32, "control/localization/minimum_fit_error">,
    pub odometry_noise: Parameter<Vector3<f32>, "control/localization/odometry_noise">,
    pub player_number: Parameter<PlayerNumber, "player_number">,
    pub score_per_good_match: Parameter<f32, "control/localization/score_per_good_match">,
    pub use_line_measurements: Parameter<bool, "control/localization/use_line_measurements">,

    pub line_data_bottom: PerceptionInput<Option<LineData>, "VisionBottom", "line_data?">,
    pub line_data_top: PerceptionInput<Option<LineData>, "VisionTop", "line_data?">,

    pub robot_to_field: PersistentState<Isometry2<f32>, "robot_to_field">,
}

#[context]
#[derive(Default)]
pub struct MainOutputs {
    pub robot_to_field: MainOutput<Option<Isometry2<f32>>>,
}

impl Localization {
    pub fn new(_context: CreationContext) -> Result<Self> {
        Ok(Self {})
    }

    pub fn cycle(&mut self, _context: CycleContext) -> Result<MainOutputs> {
        Ok(MainOutputs::default())
    }
}
