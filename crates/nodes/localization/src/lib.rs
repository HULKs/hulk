use std::{
    boxed::Box,
    collections::BTreeMap,
    future::Future,
    pin::Pin,
    sync::Arc,
    time::{Duration, SystemTime},
};

#[allow(dead_code)]
mod algorithm;

use color_eyre::Result;
use nalgebra as na;
use serde::{Deserialize, Serialize};

use booster::{FallDownState, ImuState, Odometer};
use coordinate_systems::{Field, Ground};
use geometry::line_segment::LineSegment;
use hsl_network_messages::PlayerNumber;
use linear_algebra::Isometry2;
use ros_z::{prelude::*, qos::QosDurability, time::Time};
use ros_z_streams::{CreateFutureMapBuilder, FutureItem, FutureResult};
use types::{
    field_dimensions::FieldDimensions,
    filtered_game_controller_state::FilteredGameControllerState,
    initial_pose::InitialPose,
    line_data::LineData,
    localization::{ScoredPose, Update},
    players::Players,
    primary_state::PrimaryState,
};

type LocalizationFutureInputs = (
    Option<Odometer>,
    Option<ImuState>,
    Option<FallDownState>,
    Option<LineData>,
);

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
    pub future_queue_lag: FutureQueueLagParameters,
}

#[derive(Debug, Clone, Serialize, Deserialize, Message)]
#[serde(deny_unknown_fields)]
pub struct FutureQueueLagParameters {
    pub odometer: Duration,
    pub imu_state: Duration,
    pub fall_down_state: Duration,
    pub line_data: Duration,
}

struct DebugSubscriptions {
    correspondence_lines: bool,
    fit_errors: bool,
    measured_lines_in_field: bool,
    pose_hypotheses: bool,
    updates: bool,
    gyro_movement: bool,
}

struct LocalizationPublishers {
    correspondence_lines: Publisher<Vec<LineSegment<Field>>>,
    fit_errors: Publisher<Vec<Vec<Vec<Vec<f32>>>>>,
    measured_lines_in_field: Publisher<Vec<LineSegment<Field>>>,
    pose_hypotheses: Publisher<Vec<ScoredPose>>,
    updates: Publisher<Vec<Vec<Update>>>,
    gyro_movement: Publisher<f32>,
    ground_to_field: Publisher<Option<Isometry2<Ground, Field>>>,
    is_localization_converged: Publisher<bool>,
}

pub fn run_boxed(ctx: Arc<Context>) -> Pin<Box<dyn Future<Output = Result<()>> + Send>> {
    Box::pin(run(ctx))
}

async fn run(ctx: Arc<Context>) -> Result<()> {
    let node = ctx.create_node("localization").build().await?;

    let parameters = node.bind_parameter_as::<Parameters>("localization")?;
    let initial_parameters = parameters.snapshot().typed().clone();

    let field_dimensions_cache = node
        .create_cache::<FieldDimensions>("field_dimensions", 1)?
        .with_qos(QosProfile {
            durability: QosDurability::TransientLocal,
            ..Default::default()
        })
        .build()
        .await?;
    let player_number_cache = node
        .create_cache::<PlayerNumber>("player_number", 1)?
        .with_qos(QosProfile {
            durability: QosDurability::TransientLocal,
            ..Default::default()
        })
        .build()
        .await?;
    let primary_state_cache = node
        .create_cache::<PrimaryState>("primary_state", 1)?
        .build()
        .await?;
    let filtered_game_controller_state_cache = node
        .create_cache::<FilteredGameControllerState>("filtered_game_controller_state", 1)?
        .build()
        .await?;

    let mut future_inputs = node
        .create_future_map_builder()
        .create_future_subscriber::<Odometer>(
            "inputs/odometer",
            initial_parameters.future_queue_lag.odometer,
        )
        .await?
        .create_future_subscriber::<ImuState>(
            "inputs/imu_state",
            initial_parameters.future_queue_lag.imu_state,
        )
        .await?
        .create_future_subscriber::<FallDownState>(
            "inputs/fall_down_state",
            initial_parameters.future_queue_lag.fall_down_state,
        )
        .await?
        .create_future_subscriber::<LineData>(
            "line_data",
            initial_parameters.future_queue_lag.line_data,
        )
        .await?
        .build();

    let publishers = LocalizationPublishers {
        correspondence_lines: node
            .publisher::<Vec<LineSegment<Field>>>("localization/correspondence_lines")?
            .build()
            .await?,
        fit_errors: node
            .publisher::<Vec<Vec<Vec<Vec<f32>>>>>("localization/fit_errors")?
            .build()
            .await?,
        measured_lines_in_field: node
            .publisher::<Vec<LineSegment<Field>>>("localization/measured_lines_in_field")?
            .build()
            .await?,
        pose_hypotheses: node
            .publisher::<Vec<ScoredPose>>("localization/pose_hypotheses")?
            .build()
            .await?,
        updates: node
            .publisher::<Vec<Vec<Update>>>("localization/updates")?
            .build()
            .await?,
        gyro_movement: node
            .publisher::<f32>("localization/gyro_movement")?
            .build()
            .await?,
        ground_to_field: node
            .publisher::<Option<Isometry2<Ground, Field>>>("ground_to_field")?
            .build()
            .await?,
        is_localization_converged: node
            .publisher::<bool>("is_localization_converged")?
            .build()
            .await?,
    };

    let mut localization = None;

    loop {
        let future_item = future_inputs.recv().await?;

        let Some(field_dimensions) = field_dimensions_cache.get_latest() else {
            continue;
        };
        let Some(player_number) = player_number_cache.get_latest() else {
            continue;
        };
        let Some(primary_state) = primary_state_cache.get_latest() else {
            continue;
        };

        if localization.is_none() {
            localization = Some(create_localization(&field_dimensions)?);
        }

        let parameters_snapshot = parameters.snapshot();
        let parameters = parameters_snapshot.typed();
        let filtered_game_controller_state = filtered_game_controller_state_cache.get_latest();
        let cycle_context = build_cycle_context(
            parameters,
            &field_dimensions,
            &player_number,
            &primary_state,
            filtered_game_controller_state.as_deref(),
            &future_item,
            debug_subscriptions(&publishers),
            node.clock().now(),
        );

        let Some(localization) = localization.as_mut() else {
            continue;
        };
        let outputs = localization.cycle(cycle_context)?;

        publish_outputs(&publishers, outputs).await?;
    }
}

fn collect_perception_input<T: Clone>(
    persistent: &FutureResult<LocalizationFutureInputs>,
    temporary: &FutureResult<LocalizationFutureInputs>,
    get: impl Fn(&LocalizationFutureInputs) -> Option<&T> + Copy,
) -> algorithm::PerceptionInput<T> {
    algorithm::PerceptionInput {
        persistent: collect_perception_side(persistent, get),
        temporary: collect_perception_side(temporary, get),
    }
}

fn collect_perception_side<T: Clone>(
    inputs: &FutureResult<LocalizationFutureInputs>,
    get: impl Fn(&LocalizationFutureInputs) -> Option<&T> + Copy,
) -> BTreeMap<SystemTime, Vec<T>> {
    let mut output = BTreeMap::new();
    for (time, values) in inputs {
        if let Some(value) = get(values) {
            output
                .entry(time.to_wallclock())
                .or_insert_with(Vec::new)
                .push(value.clone());
        }
    }
    output
}

#[allow(clippy::too_many_arguments)]
fn build_cycle_context<'a>(
    parameters: &'a Parameters,
    field_dimensions: &'a FieldDimensions,
    player_number: &'a PlayerNumber,
    primary_state: &'a PrimaryState,
    filtered_game_controller_state: Option<&'a FilteredGameControllerState>,
    future_item: &FutureItem<'_, LocalizationFutureInputs>,
    subscriptions: DebugSubscriptions,
    now: Time,
) -> algorithm::CycleContext<'a> {
    algorithm::CycleContext {
        correspondence_lines: algorithm::DebugOutput::new(subscriptions.correspondence_lines),
        fit_errors: algorithm::DebugOutput::new(subscriptions.fit_errors),
        measured_lines_in_field: algorithm::DebugOutput::new(subscriptions.measured_lines_in_field),
        pose_hypotheses: algorithm::DebugOutput::new(subscriptions.pose_hypotheses),
        updates: algorithm::DebugOutput::new(subscriptions.updates),
        gyro_movement: algorithm::DebugOutput::new(subscriptions.gyro_movement),
        filtered_game_controller_state,
        primary_state,
        cycle_start_time: now.to_wallclock(),
        odometer: collect_perception_input(
            &future_item.persistent,
            future_item.temporary,
            |inputs| inputs.0.as_ref(),
        ),
        imu_state: collect_perception_input(
            &future_item.persistent,
            future_item.temporary,
            |inputs| inputs.1.as_ref(),
        ),
        fall_down_state: collect_perception_input(
            &future_item.persistent,
            future_item.temporary,
            |inputs| inputs.2.as_ref(),
        ),
        line_data: collect_perception_input(
            &future_item.persistent,
            future_item.temporary,
            |inputs| inputs.3.as_ref(),
        ),
        parameters,
        field_dimensions,
        player_number,
    }
}

fn debug_subscriptions(publishers: &LocalizationPublishers) -> DebugSubscriptions {
    DebugSubscriptions {
        correspondence_lines: publishers.correspondence_lines.has_subscribers(),
        fit_errors: publishers.fit_errors.has_subscribers(),
        measured_lines_in_field: publishers.measured_lines_in_field.has_subscribers(),
        pose_hypotheses: publishers.pose_hypotheses.has_subscribers(),
        updates: publishers.updates.has_subscribers(),
        gyro_movement: publishers.gyro_movement.has_subscribers(),
    }
}

fn create_localization(
    field_dimensions: &FieldDimensions,
) -> color_eyre::Result<algorithm::Localization> {
    algorithm::Localization::new(algorithm::CreationContext { field_dimensions })
}

async fn publish_outputs(
    publishers: &LocalizationPublishers,
    outputs: algorithm::CycleOutputs,
) -> color_eyre::Result<()> {
    publishers
        .ground_to_field
        .publish(&outputs.ground_to_field)
        .await?;
    publishers
        .is_localization_converged
        .publish(&outputs.is_localization_converged)
        .await?;

    if let Some(correspondence_lines) = outputs.correspondence_lines {
        publishers
            .correspondence_lines
            .publish(&correspondence_lines)
            .await?;
    }
    if let Some(fit_errors) = outputs.fit_errors {
        publishers.fit_errors.publish(&fit_errors).await?;
    }
    if let Some(measured_lines_in_field) = outputs.measured_lines_in_field {
        publishers
            .measured_lines_in_field
            .publish(&measured_lines_in_field)
            .await?;
    }
    if let Some(pose_hypotheses) = outputs.pose_hypotheses {
        publishers.pose_hypotheses.publish(&pose_hypotheses).await?;
    }
    if let Some(updates) = outputs.updates {
        publishers.updates.publish(&updates).await?;
    }
    if let Some(gyro_movement) = outputs.gyro_movement {
        publishers.gyro_movement.publish(&gyro_movement).await?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn collect_perception_input_preserves_persistent_and_temporary_order() {
        let mut persistent = FutureResult::new();
        persistent.insert(
            Time::from_nanos(2),
            (
                Some(Odometer {
                    x: 2.0,
                    y: 0.0,
                    theta: 0.0,
                }),
                None,
                None,
                None,
            ),
        );
        let mut temporary = FutureResult::new();
        temporary.insert(
            Time::from_nanos(3),
            (
                Some(Odometer {
                    x: 3.0,
                    y: 0.0,
                    theta: 0.0,
                }),
                None,
                None,
                None,
            ),
        );

        let input = collect_perception_input(&persistent, &temporary, |inputs| inputs.0.as_ref());

        let persistent_values = input
            .persistent
            .get(&Time::from_nanos(2).to_wallclock())
            .expect("persistent odometer exists");
        let temporary_values = input
            .temporary
            .get(&Time::from_nanos(3).to_wallclock())
            .expect("temporary odometer exists");

        assert_eq!(persistent_values[0].x, 2.0);
        assert_eq!(temporary_values[0].x, 3.0);
    }
}
