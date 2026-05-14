use std::time::{Duration, SystemTime};
use std::{future::pending, sync::Arc};

use color_eyre::Result;
use nalgebra::{Matrix2, Matrix2x4, Matrix4};
use ordered_float::NotNan;
use serde::{Deserialize, Serialize};

use booster::Odometer;
use coordinate_systems::{Ground, Pixel};
use filtering::kalman_filter::KalmanFilter;
use geometry::circle::Circle;
use linear_algebra::{IntoFramed, Isometry2, distance};
use path_serde::{PathDeserialize, PathIntrospect, PathSerialize};
use projection::camera_matrix::CameraMatrix;
use ros_z::{IntoEyreResultExt, Message, context::Context, prelude::*};
use types::{
    ball_detection::BallPercept,
    ball_position::{BallPosition, HypotheticalBallPosition},
    field_dimensions::FieldDimensions,
    multivariate_normal_distribution::MultivariateNormalDistribution,
    object_detection::{Object, RobocupObjectLabel},
    parameters::BallFilterParameters,
};

mod hypothesis;

pub use hypothesis::{BallHypothesis, BallMode};

#[derive(
    Debug,
    Default,
    Clone,
    Serialize,
    Deserialize,
    PathSerialize,
    PathDeserialize,
    PathIntrospect,
    Message,
)]
pub struct BallFilter {
    pub hypotheses: Vec<BallHypothesis>,
}

impl BallFilter {
    pub fn best_hypothesis(&self, validity_threshold: f32) -> Option<&BallHypothesis> {
        self.hypotheses
            .iter()
            .filter(|hypothesis| hypothesis.validity >= validity_threshold)
            .max_by(|a, b| a.validity.partial_cmp(&b.validity).unwrap())
    }

    pub fn decay_hypotheses(&mut self, decay_factor_criterion: impl Fn(&BallHypothesis) -> f32) {
        for hypothesis in self.hypotheses.iter_mut() {
            let decay_factor = decay_factor_criterion(hypothesis);
            hypothesis.validity *= decay_factor;
        }
    }

    pub fn predict(
        &mut self,
        delta_time: Duration,
        last_to_current_odometry: Isometry2<Ground, Ground>,
        velocity_decay: f32,
        moving_process_noise: Matrix4<f32>,
        resting_process_noise: Matrix2<f32>,
        log_likelihood_of_zero_velocity_threshold: f32,
    ) {
        for hypothesis in self.hypotheses.iter_mut() {
            hypothesis.predict(
                delta_time,
                last_to_current_odometry,
                velocity_decay,
                moving_process_noise,
                resting_process_noise,
                log_likelihood_of_zero_velocity_threshold,
            )
        }
    }

    pub fn reset(&mut self) {
        self.hypotheses.clear()
    }

    pub fn remove_hypotheses(
        &mut self,
        is_valid: impl Fn(&BallHypothesis) -> bool,
        merge_criterion: impl Fn(&BallHypothesis, &BallHypothesis) -> bool,
    ) -> Vec<BallHypothesis> {
        let (valid, removed): (Vec<_>, Vec<_>) = self.hypotheses.drain(..).partition(is_valid);

        self.hypotheses = valid
            .into_iter()
            .fold(vec![], |mut deduplicated, hypothesis| {
                let mergeable_hypothesis = deduplicated
                    .iter_mut()
                    .find(|existing_hypothesis| merge_criterion(existing_hypothesis, &hypothesis));

                if let Some(mergeable_hypothesis) = mergeable_hypothesis {
                    mergeable_hypothesis.merge(hypothesis)
                } else {
                    deduplicated.push(hypothesis);
                }

                deduplicated
            });

        removed
    }

    pub fn spawn(
        &mut self,
        detection_time: SystemTime,
        measurement: MultivariateNormalDistribution<2>,
        initial_moving_covariance: Matrix4<f32>,
    ) {
        let closest_hypothesis = self.hypotheses.iter().min_by_key(|hypothesis| {
            NotNan::new(distance(
                measurement.mean.framed().as_point(),
                hypothesis.position().position,
            ))
            .expect("distance is nan")
        });

        let mut new_hypothesis = MultivariateNormalDistribution {
            mean: closest_hypothesis.map_or(
                nalgebra::vector![measurement.mean.x, measurement.mean.y, 0.0, 0.0],
                |hypothesis| {
                    let old_position = hypothesis.position().position.inner.coords;
                    nalgebra::vector![old_position.x, old_position.y, 0.0, 0.0]
                },
            ),
            covariance: initial_moving_covariance,
        };

        if closest_hypothesis.is_some() {
            KalmanFilter::update(
                &mut new_hypothesis,
                Matrix2x4::identity(),
                measurement.mean,
                measurement.covariance,
            )
        }

        let new_hypothesis = BallHypothesis {
            mode: BallMode::Moving(new_hypothesis),
            last_seen: detection_time,
            validity: 1.0,
        };

        self.hypotheses.push(new_hypothesis)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Message)]
#[serde(deny_unknown_fields)]
pub struct Parameters {
    pub field_dimensions: FieldDimensions,
    pub ball_filter_configuration: BallFilterParameters,
}

pub async fn run(ctx: Arc<Context>) -> Result<()> {
    let node = ctx.create_node("ball_filter").build().await.into_eyre()?;

    let _parameters = node
        .bind_parameter_as::<Parameters>("ball_filter")
        .into_eyre()?;
    let _historic_camera_matrix_sub = node
        .subscriber::<CameraMatrix>("camera_matrix")
        .into_eyre()?
        .build()
        .await
        .into_eyre()?;
    let _integrated_odometry_sub = node
        .subscriber::<Odometer>("inputs/odometer")
        .into_eyre()?
        .build()
        .await
        .into_eyre()?;
    let _detected_objects_sub = node
        .subscriber::<Vec<Object<RobocupObjectLabel>>>("detected_objects")
        .into_eyre()?
        .build()
        .await
        .into_eyre()?;
    let _filter_state_pub = node
        .publisher::<BallFilter>("ball_filter_state")
        .into_eyre()?
        .build()
        .await
        .into_eyre()?;
    let _best_ball_hypothesis_pub = node
        .publisher::<BallHypothesis>("best_ball_hypothesis")
        .into_eyre()?
        .build()
        .await
        .into_eyre()?;
    let _filtered_balls_in_image_pub = node
        .publisher::<Vec<Circle<Pixel>>>("filtered_balls_in_image")
        .into_eyre()?
        .build()
        .await
        .into_eyre()?;
    let _ball_percepts_pub = node
        .publisher::<Vec<BallPercept>>("ball_percepts")
        .into_eyre()?
        .build()
        .await
        .into_eyre()?;
    let _ball_position_pub = node
        .publisher::<BallPosition<Ground>>("ball_position")
        .into_eyre()?
        .build()
        .await
        .into_eyre()?;
    let _hypothetical_ball_positions_pub = node
        .publisher::<Vec<HypotheticalBallPosition<Ground>>>("hypothetical_ball_positions")
        .into_eyre()?
        .build()
        .await
        .into_eyre()?;

    pending::<()>().await;

    Ok(())
}
