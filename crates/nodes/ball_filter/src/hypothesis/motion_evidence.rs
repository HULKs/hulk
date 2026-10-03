//! Confirm motion from associated observations, without interpreting robot motion
//! or an unknown displacement during occlusion as ball velocity.
use std::time::Duration;

use coordinate_systems::Ground;
use linear_algebra::Isometry2;
use nalgebra::{Matrix2, Matrix4, Vector2};
use ros_z::{Message, time::Time};
use serde::{Deserialize, Serialize};
use types::{
    multivariate_normal_distribution::MultivariateNormalDistribution, time_wrapper::TimeWrapper,
};

const MINIMUM_INTERVAL: Duration = Duration::from_millis(20);
const MAXIMUM_INTERVAL: Duration = Duration::from_millis(200);
const SIGNIFICANCE_SQUARED: f32 = 9.0;
// A millimetre of uncertainty prevents near-exact synthetic observations from
// converting numerical roundoff or submillimetre motion into decisive evidence.
const MINIMUM_POSITION_VARIANCE: f32 = 1e-6;

#[derive(Clone, Debug, Default, Serialize, Deserialize, Message)]
pub struct MotionEvidence {
    observations: Vec<TimeWrapper<MultivariateNormalDistribution<2>>>,
}

impl MotionEvidence {
    pub(super) fn transform(&mut self, old_to_current: Isometry2<Ground, Ground>) {
        let rotation = old_to_current.inner.rotation.to_rotation_matrix();
        let rotation = rotation.matrix();
        let translation = old_to_current.inner.translation.vector;
        for observation in &mut self.observations {
            observation.inner.mean = rotation * observation.inner.mean + translation;
            observation.inner.covariance =
                rotation * observation.inner.covariance * rotation.transpose();
        }
    }

    pub(super) fn observe(
        &mut self,
        time: Time,
        mut measurement: MultivariateNormalDistribution<2>,
    ) -> Option<MultivariateNormalDistribution<4>> {
        if !measurement.mean.iter().all(|value| value.is_finite())
            || !measurement.covariance.iter().all(|value| value.is_finite())
        {
            self.observations.clear();
            return None;
        }
        measurement.covariance = (measurement.covariance + measurement.covariance.transpose())
            * 0.5
            + Matrix2::identity() * MINIMUM_POSITION_VARIANCE;
        if measurement.covariance.cholesky().is_none() {
            self.observations.clear();
            return None;
        }
        if let Some(previous) = self.observations.last() {
            if time <= previous.time {
                return None;
            }
            let interval = time.duration_since(previous.time);
            if interval < MINIMUM_INTERVAL {
                return None;
            }
            if interval > MAXIMUM_INTERVAL {
                self.observations.clear();
            }
        }
        self.observations.push(TimeWrapper {
            time,
            inner: measurement,
        });
        if self.observations.len() > 3 {
            self.observations.remove(0);
        }
        let [first, middle, last] = self.observations.as_slice() else {
            return None;
        };
        let first_dt = middle.time.duration_since(first.time).as_secs_f32();
        let second_dt = last.time.duration_since(middle.time).as_secs_f32();
        let first_displacement = middle.inner.mean - first.inner.mean;
        let second_displacement = last.inner.mean - middle.inner.mean;
        if first_displacement.dot(&second_displacement) <= 0.0
            || squared_distance(
                first_displacement,
                first.inner.covariance + middle.inner.covariance,
            )? <= SIGNIFICANCE_SQUARED
            || squared_distance(
                second_displacement,
                middle.inner.covariance + last.inner.covariance,
            )? <= SIGNIFICANCE_SQUARED
        {
            return None;
        }
        let velocity_difference = second_displacement / second_dt - first_displacement / first_dt;
        // The middle observation enters the two velocities with opposite signs.
        let difference_covariance = first.inner.covariance / first_dt.powi(2)
            + middle.inner.covariance * (first_dt.recip() + second_dt.recip()).powi(2)
            + last.inner.covariance / second_dt.powi(2);
        if squared_distance(velocity_difference, difference_covariance)? > SIGNIFICANCE_SQUARED {
            return None;
        }
        let span = first_dt + second_dt;
        let velocity = (last.inner.mean - first.inner.mean) / span;
        let mut covariance = Matrix4::zeros();
        covariance
            .fixed_view_mut::<2, 2>(0, 0)
            .copy_from(&last.inner.covariance);
        covariance
            .fixed_view_mut::<2, 2>(0, 2)
            .copy_from(&(last.inner.covariance / span));
        covariance
            .fixed_view_mut::<2, 2>(2, 0)
            .copy_from(&(last.inner.covariance / span));
        covariance
            .fixed_view_mut::<2, 2>(2, 2)
            .copy_from(&((first.inner.covariance + last.inner.covariance) / span.powi(2)));
        // The newest position also appears in the velocity estimate: retaining
        // their correlation is essential for the next Kalman prediction/update.
        covariance.cholesky()?;
        Some(MultivariateNormalDistribution {
            mean: nalgebra::vector![last.inner.mean.x, last.inner.mean.y, velocity.x, velocity.y],
            covariance,
        })
    }
}

fn squared_distance(displacement: Vector2<f32>, covariance: Matrix2<f32>) -> Option<f32> {
    let distance = displacement.dot(&covariance.cholesky()?.solve(&displacement));
    distance.is_finite().then_some(distance)
}
