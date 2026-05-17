use std::{future::pending, sync::Arc, time::Duration};

use color_eyre::Result;
use nalgebra as na;
use serde::{Deserialize, Serialize};

use booster::{FallDownState, ImuState, Odometer};
use coordinate_systems::{Field, Ground};
use geometry::line_segment::LineSegment;
use hsl_network_messages::PlayerNumber;
use linear_algebra::Isometry2;
use ros_z::{prelude::*, qos::QosDurability};
use types::{
    field_dimensions::FieldDimensions,
    filtered_game_controller_state::FilteredGameControllerState,
    initial_pose::InitialPose,
    line_data::LineData,
    localization::{ScoredPose, Update},
    players::Players,
    primary_state::PrimaryState,
};

#[derive(Debug, Clone, Serialize, Deserialize, Message)]
#[serde(deny_unknown_fields)]
pub struct Parameters {
    pub circle_measurement_noise: na::Vector2<f32>,
    pub good_matching_threshold: f32,
    pub gradient_convergence_threshold: f32,
    pub gradient_descent_step_size: f32,
    pub hypothesis_prediction_score_reduction_factor: f32,
    pub hypothesis_retain_factor: f32,
    pub hypothesis_score_base_increase: f32,
    pub initial_hypothesis_covariance: na::Matrix3<f32>,
    pub initial_hypothesis_score: f32,
    pub initial_poses: Players<InitialPose>,
    pub line_length_acceptance_factor: f32,
    pub line_measurement_noise: na::Vector2<f32>,
    pub additional_moving_noise_line: na::Vector2<f32>,
    pub additional_moving_noise_circle: na::Vector2<f32>,
    pub maximum_amount_of_gradient_descent_iterations: usize,
    pub maximum_amount_of_outer_iterations: usize,
    pub minimum_fit_error: f32,
    pub odometry_noise: na::Vector3<f32>,
    pub penalized_distance: f32,
    pub penalized_hypothesis_covariance: na::Matrix3<f32>,
    pub score_per_good_match: f32,
    pub tentative_penalized_duration: Duration,
    pub use_line_measurements: bool,
}

pub async fn run(ctx: Arc<Context>) -> Result<()> {
    let node = ctx.create_node("localization").build().await?;

    let _parameters = node.bind_parameter_as::<Parameters>("localization")?;
    let _filtered_game_controller_state_sub = node
        .subscriber::<Option<FilteredGameControllerState>>("filtered_game_controller_state")?
        .build()
        .await?;
    let _primary_state_sub = node
        .subscriber::<PrimaryState>("primary_state")?
        .build()
        .await?;
    let _odometer_sub = node
        .subscriber::<Odometer>("inputs/odometer")?
        .build()
        .await?;
    let _fall_down_state_sub = node
        .subscriber::<FallDownState>("inputs/fall_down_state")?
        .build()
        .await?;
    let _imu_state_sub = node
        .subscriber::<ImuState>("inputs/imu_state")?
        .build()
        .await?;
    let _line_data_sub = node.subscriber::<LineData>("line_data")?.build().await?;
    let _field_dimensions_sub = node
        .subscriber::<FieldDimensions>("field_dimensions")?
        .qos(QosProfile {
            durability: QosDurability::TransientLocal,
            ..Default::default()
        })
        .build()
        .await?;
    let _player_number_sub = node
        .subscriber::<PlayerNumber>("player_number")?
        .qos(QosProfile {
            durability: QosDurability::TransientLocal,
            ..Default::default()
        })
        .build()
        .await?;
    let _correspondence_lines_pub = node
        .publisher::<Vec<LineSegment<Field>>>("localization/correspondence_lines")?
        .build()
        .await?;
    let _fit_errors_pub = node
        .publisher::<Vec<Vec<Vec<Vec<f32>>>>>("localization/fit_errors")?
        .build()
        .await?;
    let _measured_lines_in_field_pub = node
        .publisher::<Vec<LineSegment<Field>>>("localization/measured_lines_in_field")?
        .build()
        .await?;
    let _pose_hypotheses_pub = node
        .publisher::<Vec<ScoredPose>>("localization/pose_hypotheses")?
        .build()
        .await?;
    let _updates_pub = node
        .publisher::<Vec<Vec<Update>>>("localization/updates")?
        .build()
        .await?;
    let _gyro_movement_pub = node
        .publisher::<f32>("localization/gyro_movement")?
        .build()
        .await?;
    let _ground_to_field_pub = node
        .publisher::<Isometry2<Ground, Field>>("ground_to_field")?
        .build()
        .await?;
    let _is_localization_converged_pub = node
        .publisher::<bool>("is_localization_converged")?
        .build()
        .await?;

    pending::<()>().await;

    Ok(())
}
