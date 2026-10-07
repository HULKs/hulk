use super::{Estimator, control_keys};
use color_eyre::{Result, eyre::eyre};
use fagra::StateKey;
use localization_fagra::{
    factors::{FieldContainment, MotionPrior},
    spline::PoseSpline,
    variables::PoseControl,
};
use ros_z::time::Time;

impl Estimator {
    pub(super) fn check_time(
        &self,
        time: Time,
        sensor: &'static str,
    ) -> Result<Option<(i64, f64)>> {
        if time < self.origin
            || time.as_nanos()
                < self
                    .latest_time
                    .as_nanos()
                    .saturating_sub(self.parameters.timing.window_ns())
        {
            tracing::warn!(
                sensor,
                ?time,
                "discarding measurement outside optimization window or epoch"
            );
            return Ok(None);
        }
        self.segment_and_tau(time).map(Some)
    }

    pub(super) fn commit_time(&mut self, time: Time) {
        self.latest_time = self.latest_time.max(time);
    }

    pub(super) fn segment_and_tau(&self, time: Time) -> Result<(i64, f64)> {
        let elapsed = time
            .as_nanos()
            .checked_sub(self.origin.as_nanos())
            .filter(|elapsed| *elapsed >= 0)
            .ok_or_else(|| eyre!("timestamp outside trajectory domain"))?;
        Ok((
            elapsed / self.parameters.timing.knot_ns(),
            (elapsed % self.parameters.timing.knot_ns()) as f64
                / self.parameters.timing.knot_ns() as f64,
        ))
    }

    pub(super) fn ensure_segment(&mut self, segment: i64) -> Result<[StateKey<PoseControl>; 4]> {
        let largest = *self
            .controls
            .last_key_value()
            .ok_or_else(|| eyre!("empty trajectory"))?
            .0;
        let next_segment = self.segments().end;
        let gap = segment - next_segment >= self.parameters.model.prediction_gap_segments;
        for index in largest + 1..=segment + 2 {
            let previous = self.graph.get(self.controls[&(index - 1)])?;
            let before = self.graph.get(self.controls[&(index - 2)])?;
            let mut prediction = previous.clone();
            if !gap {
                prediction.pose.inner.translation.vector +=
                    previous.pose.inner.translation.vector - before.pose.inner.translation.vector;
                prediction.pose.inner.rotation *=
                    before.pose.inner.rotation.inverse() * previous.pose.inner.rotation;
            }
            prediction.pose.inner.rotation.renormalize();
            self.controls.insert(index, self.graph.add(prediction));
        }
        for current in next_segment..=segment {
            self.add_motion_prior(current, gap && current < segment)?;
            self.add_containment(current)?;
        }
        control_keys(&self.controls, segment)
    }

    pub(super) fn add_motion_prior(&mut self, segment: i64, gap: bool) -> Result<()> {
        let root = MotionPrior::information_root(
            self.parameters.timing.trajectory_spacing.as_secs_f64(),
            self.parameters.model.rotation_process_variance,
            self.parameters.accelerometer_process_noise_variance,
        )? / if gap {
            self.parameters.model.gap_uncertainty_multiplier.sqrt()
        } else {
            1.0
        };
        self.graph.add_factor(MotionPrior {
            controls: control_keys(&self.controls, segment)?,
            duration: self.parameters.timing.trajectory_spacing.as_secs_f64(),
            information_root: root,
            use_start_velocity: !gap,
        })?;
        Ok(())
    }

    pub(super) fn add_containment(&mut self, segment: i64) -> Result<()> {
        if let Some(alignment) = self.alignment {
            self.graph.add_factor(FieldContainment {
                controls: control_keys(&self.controls, segment)?,
                duration: self.parameters.timing.trajectory_spacing.as_secs_f64(),
                tau: self.parameters.model.containment_tau,
                alignment,
                half_extents: self.field_half_extents,
                sigma: self.parameters.field_containment_sigma,
            })?;
        }
        Ok(())
    }

    pub(super) fn spline(&self, controls: [StateKey<PoseControl>; 4]) -> Result<PoseSpline<f64>> {
        Ok(PoseSpline::new(
            [
                self.graph.get(controls[0])?,
                self.graph.get(controls[1])?,
                self.graph.get(controls[2])?,
                self.graph.get(controls[3])?,
            ],
            self.parameters.timing.trajectory_spacing.as_secs_f64(),
        )?)
    }

    pub(super) fn retire_old_segments(&mut self) -> Result<()> {
        let oldest = self.oldest_window_segment();
        let mut old_states: Vec<_> = self
            .controls
            .range(..oldest - 1)
            .map(|(_, key)| key.block_id())
            .collect();
        if old_states.is_empty() {
            return Ok(());
        }
        let oldest_bias =
            oldest * self.parameters.timing.knot_ns() / self.parameters.timing.bias_ns();
        old_states.extend(
            self.biases
                .range(..oldest_bias)
                .map(|(_, key)| key.block_id()),
        );
        self.graph.marginalize(&old_states)?;
        self.biases.retain(|index, _| *index >= oldest_bias);
        self.controls.retain(|index, _| *index >= oldest - 1);
        self.measurements.retain(|index, _| *index >= oldest);
        self.yaw_factors.retain(|index, _| *index >= oldest);
        self.retire_preintegration(oldest);
        while let Some(entry) = self.foot_batches.first_entry() {
            if *entry.key() >= oldest {
                break;
            }
            self.graph.remove_batch(entry.remove())?;
        }
        while let Some(entry) = self.odometry_batches.first_entry() {
            if *entry.key() >= oldest {
                break;
            }
            self.graph.remove_batch(entry.remove())?;
        }
        for &(index, batch) in &self.reprojection_batches {
            if index < oldest {
                self.graph.remove_batch(batch)?;
            }
        }
        self.reprojection_batches
            .retain(|(index, _)| *index >= oldest);
        Ok(())
    }

    pub(super) fn oldest_window_segment(&self) -> i64 {
        // Round the actual cutoff down, not the window duration. A fractional
        // window can still admit measurements in the preceding segment.
        (self.latest_time.as_nanos() - self.origin.as_nanos())
            .saturating_sub(self.parameters.timing.window_ns())
            .div_euclid(self.parameters.timing.knot_ns())
            .max(0)
    }
}
