use color_eyre::Result;
use localization_fagra::factors::{AdjacentKinematicOdometry, KinematicOdometry};
use nalgebra::Matrix2;
use ros_z::time::Time;
use types::odometry::KinematicOdometryDelta;

use super::{Estimator, recovery::MotionRecord};

impl Estimator {
    pub(crate) fn ingest_kinematic_odometry(
        &mut self,
        sample: KinematicOdometryDelta,
    ) -> Result<bool> {
        let Some(noise) = self.parameters.kinematic_odometry_noise.as_ref() else {
            return Ok(false);
        };
        if sample.time <= sample.previous_time
            || self
                .latest_kinematic_time
                .is_some_and(|last| sample.previous_time < last)
            || !sample
                .current_to_previous
                .inner
                .to_homogeneous()
                .iter()
                .all(|v| v.is_finite())
            || sample.previous_time.as_nanos()
                < self
                    .latest_time
                    .max(sample.time)
                    .as_nanos()
                    .saturating_sub(self.parameters.timing.window_ns())
        {
            return Ok(false);
        }
        let Some((a, _)) = self.check_time(sample.previous_time, "kinematic odometry")? else {
            return Ok(false);
        };
        let Some((b, _)) = self.check_time(sample.time, "kinematic odometry")? else {
            return Ok(false);
        };
        if b - a > 1 {
            return Ok(false);
        }
        let dt = sample
            .time
            .duration_since(sample.previous_time)
            .as_secs_f64();
        // ponytail: diagonal noise approximates shared encoder/IMU errors; calibrate
        // on recordings, use correlated preintegration if consistency requires it.
        let variance =
            noise.position_sigma.map(|s| s * s) + noise.translation_variance_per_second * dt;
        let information_root = Matrix2::from_diagonal(&variance.map(|v| v.sqrt().recip()));
        let translation = linear_algebra::Vector2::wrap(
            sample
                .current_to_previous
                .translation()
                .coords()
                .inner
                .cast(),
        );
        self.accept_motion(MotionRecord::Kinematic {
            previous_time: sample.previous_time,
            time: sample.time,
            translation,
            information_root,
        })?;
        self.latest_kinematic_time = Some(sample.time);
        Ok(true)
    }

    pub(super) fn insert_kinematic_odometry(
        &mut self,
        previous_time: Time,
        time: Time,
        translation: linear_algebra::Vector2<coordinate_systems::Ground, f64>,
        information_root: Matrix2<f64>,
    ) -> Result<()> {
        let (a, previous_tau) = self.segment_and_tau(previous_time)?;
        let (b, current_tau) = self.segment_and_tau(time)?;
        let first = self.ensure_segment(a)?;
        let second = self.ensure_segment(b)?;
        if a == b {
            self.graph.add_factor(KinematicOdometry {
                controls: first,
                duration: self.parameters.timing.trajectory_spacing.as_secs_f64(),
                previous_tau,
                current_tau,
                translation,
                information_root,
                huber_threshold: self.parameters.model.huber_threshold,
            })?;
        } else {
            self.graph.add_factor(AdjacentKinematicOdometry {
                controls: [first[0], first[1], first[2], first[3], second[3]],
                duration: self.parameters.timing.trajectory_spacing.as_secs_f64(),
                previous_tau,
                current_tau,
                translation,
                information_root,
                huber_threshold: self.parameters.model.huber_threshold,
            })?;
        }
        *self.measurements.entry(a).or_default() += 1;
        Ok(())
    }
}
