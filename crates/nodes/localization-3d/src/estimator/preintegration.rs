use std::collections::{BTreeMap, BTreeSet};

use super::{Estimator, bias::bias_segment_and_tau};
use crate::parameters::TimingParameters;
use color_eyre::{Result, eyre::WrapErr};
use coordinate_systems::{ImuReference, Robot};
use fagra::FactorKey;
use itertools::Itertools;
use linear_algebra::Vector3;
use localization_fagra::{
    factors::{ImuKinematics, PreintegratedImu},
    preintegration::{ImuNoise, ImuPreintegrator},
    variables::ImuBias,
};
use nalgebra::Matrix3;
use ros_z::time::Time;

#[derive(Clone, Copy)]
struct Sample {
    gyro: Vector3<Robot, f64>,
    force: Vector3<Robot, f64>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use booster::ImuState;
    use std::time::Duration;

    fn ingest(estimator: &mut Estimator, millis: u64, force: f32) {
        estimator
            .ingest_imu(
                estimator.origin + Duration::from_millis(millis),
                ImuState {
                    linear_acceleration: Vector3::wrap(nalgebra::vector![0.0, 0.0, force]),
                    ..Default::default()
                },
            )
            .unwrap();
    }

    #[test]
    fn impulses_missing_force_gaps_and_late_bridges_keep_distinct_likelihoods() {
        let mut estimator = super::super::tests::estimator();
        estimator.parameters.accelerometer = Some(Default::default());
        for millis in [0, 10, 20, 30, 70, 80, 90, 100] {
            ingest(
                &mut estimator,
                millis,
                match millis {
                    10 => 100.0,
                    20 => f32::NAN,
                    _ => 0.0,
                },
            );
            estimator.prepare_preintegration().unwrap();
        }
        let pieces = &estimator.preintegration.intervals[&0].pieces;
        assert!(
            (pieces.iter().map(|(_, _, p)| p.delta.duration).sum::<f64>() - 0.06).abs() < 1e-12
        );
        assert!(
            (pieces
                .iter()
                .filter(|(_, _, p)| p.acceleration)
                .map(|(_, _, p)| p.delta.duration)
                .sum::<f64>()
                - 0.05)
                .abs()
                < 1e-12
        );
        assert!(
            (pieces
                .iter()
                .map(|(_, _, p)| p.delta.velocity.z())
                .sum::<f64>()
                - 1.0)
                .abs()
                < 1e-12
        );
        for millis in [50, 60] {
            ingest(&mut estimator, millis, 0.0);
        }
        estimator.prepare_preintegration().unwrap();
        let pieces = &estimator.preintegration.intervals[&0].pieces;
        assert!((pieces.iter().map(|(_, _, p)| p.delta.duration).sum::<f64>() - 0.1).abs() < 1e-12);
        assert_eq!(pieces.len(), 3);
        assert!(!pieces[1].2.acceleration);
        assert_eq!(pieces[2].2.delta.velocity, Vector3::zeros()); // Flight, not missing data.
        let mut parameters = estimator.parameters.clone();
        parameters.accelerometer = None;
        assert!(estimator.update_parameters(parameters).is_err());
        estimator.prepare_preintegration().unwrap();
        assert_eq!(estimator.preintegration.intervals[&0].pieces.len(), 3);
    }

    #[test]
    fn bias_boundary_and_accepted_bias_refresh_reintegrate_the_right_knots() {
        let mut estimator = super::super::tests::estimator();
        estimator.parameters.accelerometer = Some(Default::default());
        for millis in (4950..=5050).step_by(2) {
            ingest(&mut estimator, millis, 9.81);
        }
        estimator.prepare_preintegration().unwrap();
        assert!(
            (estimator.preintegration.intervals[&49].pieces[0]
                .2
                .delta
                .duration
                - 0.05)
                .abs()
                < 1e-12
        );
        assert!(
            (estimator.preintegration.intervals[&50].pieces[0]
                .2
                .delta
                .duration
                - 0.05)
                .abs()
                < 1e-12
        );
        for &key in estimator.biases.values() {
            estimator
                .graph
                .set(
                    key,
                    ImuBias {
                        gyroscope: Vector3::wrap(nalgebra::vector![0.0, 0.0, 0.02]),
                        accelerometer: Vector3::wrap(nalgebra::vector![0.0, 0.0, 0.2]),
                    },
                )
                .unwrap();
        }
        estimator.prepare_preintegration().unwrap();
        for index in [49, 50] {
            let p = &estimator.preintegration.intervals[&index].pieces[0].2;
            assert!((p.delta.rotation.inner.scaled_axis().z + 0.001).abs() < 1e-12);
            assert!((p.delta.velocity.z() - (9.81_f32 as f64 - 0.2) * 0.05).abs() < 1e-12);
            assert_eq!(p.delta.reference_biases[0].gyroscope.z(), 0.02);
        }
        let mut parameters = estimator.parameters.clone();
        parameters.accelerometer.as_mut().unwrap().scale.z = 2.0;
        assert!(estimator.update_parameters(parameters).is_err());
        estimator.prepare_preintegration().unwrap();
        assert!(
            (estimator.preintegration.intervals[&50].pieces[0]
                .2
                .delta
                .velocity
                .z()
                - (9.81_f32 as f64 - 0.2) * 0.05)
                .abs()
                < 1e-12
        );
    }
}

struct Interval {
    factors: Vec<FactorKey<PreintegratedImu>>,
    boundaries: Vec<FactorKey<ImuKinematics>>,
    reference_biases: [ImuBias; 2],
    pieces: Vec<(Time, Time, ImuPreintegrator)>,
    reusable: bool,
}

pub(super) struct ImuIntervals {
    // Keep the original partition with the cache, including during recovery replay.
    interval_ns: i64,
    max_gap: std::time::Duration,
    samples: BTreeMap<Time, Sample>,
    intervals: BTreeMap<i64, Interval>,
    dirty: BTreeSet<i64>,
    terminal: Option<(i64, FactorKey<ImuKinematics>)>,
}

impl ImuIntervals {
    pub(super) fn new(timing: &TimingParameters) -> Self {
        Self {
            interval_ns: timing.interval_ns(),
            max_gap: timing.max_imu_gap,
            samples: BTreeMap::new(),
            intervals: BTreeMap::new(),
            dirty: BTreeSet::new(),
            terminal: None,
        }
    }
    pub(super) fn restore_boundary(&mut self, origin: Time, start: Time, source: &Self) {
        if let Some((&time, &sample)) = source.samples.range(..start).next_back() {
            self.insert(origin, time, sample.gyro, sample.force);
        }
    }
    #[cfg(test)]
    pub(super) fn interval_count(&self) -> usize {
        self.intervals.len()
    }
    pub(super) fn insert(
        &mut self,
        origin: Time,
        time: Time,
        gyro: Vector3<Robot, f64>,
        force: Vector3<Robot, f64>,
    ) {
        // A late/replaced reading changes the two adjacent held-measurement spans.
        // Long gaps carry no inertial likelihood, and need no empty interval cache.
        let before = self.samples.range(..time).next_back().map(|(&t, _)| t);
        let after = self
            .samples
            .range((std::ops::Bound::Excluded(time), std::ops::Bound::Unbounded))
            .next()
            .map(|(&t, _)| t);
        if self
            .samples
            .last_key_value()
            .is_some_and(|(&latest, _)| time <= latest)
        {
            let first = (before.unwrap_or(time).as_nanos() - origin.as_nanos()) / self.interval_ns;
            let last = (after.unwrap_or(time).as_nanos() - origin.as_nanos()) / self.interval_ns;
            for (_, interval) in self.intervals.range_mut(first..=last) {
                interval.reusable = false;
            }
        }
        self.samples.insert(time, Sample { gyro, force });
        self.dirty
            .insert((time.as_nanos() - origin.as_nanos()) / self.interval_ns);
        for (a, b) in before
            .map(|a| (a, time))
            .into_iter()
            .chain(after.map(|b| (time, b)))
        {
            self.dirty
                .insert((a.as_nanos() - origin.as_nanos()) / self.interval_ns);
            if b.duration_since(a) <= self.max_gap {
                let first = (a.as_nanos() - origin.as_nanos()) / self.interval_ns;
                let last = (b.as_nanos() - origin.as_nanos() - 1) / self.interval_ns;
                self.dirty.extend(first..=last);
            }
        }
    }
}

impl Estimator {
    pub(super) fn prepare_preintegration(&mut self) -> Result<()> {
        let p = &self.parameters.imu_preintegration;
        for (&index, interval) in &mut self.preintegration.intervals {
            let knot =
                index * self.parameters.timing.interval_ns() / self.parameters.timing.bias_ns();
            for (offset, reference) in interval.reference_biases.iter().enumerate() {
                let bias = self.graph.get(self.biases[&(knot + offset as i64)])?;
                if (bias.gyroscope - reference.gyroscope).norm()
                    > p.gyroscope_reintegration_threshold
                    || (bias.accelerometer - reference.accelerometer).norm()
                        > p.accelerometer_reintegration_threshold
                {
                    self.preintegration.dirty.insert(index);
                    interval.reusable = false;
                }
            }
        }
        let oldest = self.segments().start * self.parameters.timing.knot_ns()
            / self.parameters.timing.interval_ns();
        while let Some(&index) = self.preintegration.dirty.first() {
            if index >= oldest {
                self.rebuild_imu_interval(index)?;
            }
            self.preintegration.dirty.remove(&index);
        }
        if let Some((_, boundary)) = self.preintegration.terminal.take() {
            self.graph.remove_factor(boundary)?;
        }
        if let Some((&time, &sample)) = self.preintegration.samples.last_key_value()
            && self.segment_and_tau(time)?.0 >= self.segments().start
        {
            // This sample has no following span yet. Remove its instantaneous
            // gyro constraint before the next solve, when it enters an integral.
            let has_tilt = self
                .preintegration
                .intervals
                .values()
                .any(|i| i.pieces.last().is_some_and(|(_, end, _)| *end == time));
            let tilt = if has_tilt {
                0.0
            } else {
                self.parameters.imu_preintegration.tilt_sigma.recip()
            };
            let batch = self.add_imu_boundary(
                time,
                sample.gyro,
                self.parameters
                    .imu_preintegration
                    .terminal_gyroscope_sigma
                    .recip(),
                tilt,
                false,
            )?;
            self.preintegration.terminal = Some((self.segment_and_tau(time)?.0, batch));
        }
        Ok(())
    }

    fn rebuild_imu_interval(&mut self, index: i64) -> Result<()> {
        let start =
            Time::from_nanos(self.origin.as_nanos() + index * self.parameters.timing.interval_ns());
        let end = Time::from_nanos(start.as_nanos() + self.parameters.timing.interval_ns());
        let before = self
            .preintegration
            .samples
            .range(..=start)
            .next_back()
            .map_or(start, |(&t, _)| t);
        let after = self
            .preintegration
            .samples
            .range(end..)
            .next()
            .map_or(end, |(&t, _)| t);
        let (bias_keys, _) = self.ensure_biases(start)?;
        let cached = self
            .preintegration
            .intervals
            .get(&index)
            .filter(|i| i.reusable);
        let reference_biases = if let Some(cached) = cached {
            cached.reference_biases.clone()
        } else {
            [
                self.graph.get(bias_keys[0])?.clone(),
                self.graph.get(bias_keys[1])?.clone(),
            ]
        };
        let p = &self.parameters.imu_preintegration;
        let noise = ImuNoise {
            gyroscope: p.gyroscope_noise_density.powi(2),
            accelerometer: self
                .parameters
                .accelerometer
                .as_ref()
                .map_or(0.0, |a| a.noise_density.powi(2)),
            integration: p.integration_noise_density.powi(2),
        };
        let mut pieces = cached.map_or_else(Vec::new, |i| i.pieces.clone());
        let resume = pieces.last().map_or(start, |(_, end, _)| *end);
        let mut gap_boundaries = Vec::new();
        for ((&a, sample), (&b, _)) in self
            .preintegration
            .samples
            .range(before..=after)
            .tuple_windows()
        {
            if b.duration_since(a) > self.parameters.timing.max_imu_gap {
                if a >= start && a < end {
                    gap_boundaries.push((a, sample.gyro));
                }
                continue;
            }
            let left = a.max(resume);
            let right = b.min(end);
            if right <= left {
                continue;
            }
            let force = self.parameters.accelerometer.as_ref().and_then(|p| {
                let force = (sample.force.inner - p.bias.inner).component_mul(&p.scale);
                force
                    .iter()
                    .all(|v| v.is_finite())
                    .then(|| Vector3::wrap(force))
            });
            let acceleration = force.is_some();
            if !pieces
                .last()
                .is_some_and(|(_, last, p)| *last == left && p.acceleration == acceleration)
            {
                pieces.push((
                    left,
                    left,
                    ImuPreintegrator::new(reference_biases.clone(), acceleration),
                ));
            }
            let (_, last, integrated) = pieces.last_mut().unwrap();
            // Midpoint quadrature preserves the independent linearly varying bias.
            let tau = bias_segment_and_tau(self.origin, left, self.parameters.timing.bias_ns()).1
                + (right.as_nanos() - left.as_nanos()) as f64 * 0.5
                    / self.parameters.timing.bias_ns() as f64;
            integrated
                .integrate(
                    sample.gyro,
                    force,
                    right.duration_since(left).as_secs_f64(),
                    tau,
                    noise,
                )
                .wrap_err_with(|| {
                    format!("IMU propagation in interval {index}, {left:?}..{right:?}")
                })?;
            *last = right;
        }
        // Prepare all numerical work before replacing graph factors.
        let segment =
            index * self.parameters.timing.interval_ns() / self.parameters.timing.knot_ns();
        let controls = self.ensure_segment(segment)?;
        let position = self
            .parameters
            .accelerometer
            .as_ref()
            .map_or(Vector3::zeros(), |p| p.position);
        let segment_start = self.origin.as_nanos() + segment * self.parameters.timing.knot_ns();
        let mut factors = Vec::new();
        let mut covered = 0.0;
        let last = pieces.last().map(|(_, end, _)| *end);
        for (a, b, integrated) in &pieces {
            covered += integrated.delta.duration;
            factors.push(PreintegratedImu {
                controls,
                biases: bias_keys,
                duration: self.parameters.timing.trajectory_spacing.as_secs_f64(),
                start_tau: (a.as_nanos() - segment_start) as f64
                    / self.parameters.timing.knot_ns() as f64,
                end_tau: (b.as_nanos() - segment_start) as f64
                    / self.parameters.timing.knot_ns() as f64,
                information: integrated.information().wrap_err_with(|| {
                    format!("IMU covariance whitening in interval {index}, {a:?}..{b:?}")
                })?,
                delta: integrated.delta.clone(),
                gravity_compensation: Vector3::wrap(nalgebra::Vector3::new(
                    0.0,
                    0.0,
                    self.parameters.model.gravity,
                )),
                position,
            });
        }
        if let Some(old) = self.preintegration.intervals.remove(&index) {
            for key in old.factors {
                self.graph.remove_factor(key)?;
            }
            for boundary in old.boundaries {
                self.graph.remove_factor(boundary)?;
            }
        }
        let factors = factors
            .into_iter()
            .map(|factor| self.graph.add_factor(factor))
            .collect::<std::result::Result<Vec<_>, _>>()?;
        let mut boundaries = Vec::new();
        if let Some(time) = last {
            boundaries.push(
                self.add_imu_boundary(
                    time,
                    Vector3::zeros(),
                    0.0,
                    (covered
                        / self
                            .parameters
                            .imu_preintegration
                            .reference_duration
                            .as_secs_f64())
                    .sqrt()
                        / self.parameters.imu_preintegration.tilt_sigma,
                    true,
                )?,
            );
        }
        for (time, gyro) in gap_boundaries {
            // No held span consumes this gyro reading. Keep its instantaneous
            // evidence instead of fabricating motion across the missing data.
            boundaries.push(
                self.add_imu_boundary(
                    time,
                    gyro,
                    self.parameters
                        .imu_preintegration
                        .terminal_gyroscope_sigma
                        .recip(),
                    if last == Some(time) {
                        0.0
                    } else {
                        self.parameters.model.gap_tilt_sigma.recip()
                    },
                    false,
                )?,
            );
        }
        self.preintegration.intervals.insert(
            index,
            Interval {
                factors,
                boundaries,
                reference_biases,
                pieces,
                reusable: true,
            },
        );
        Ok(())
    }

    fn add_imu_boundary(
        &mut self,
        time: Time,
        gyro: Vector3<Robot, f64>,
        gyro_root: f64,
        tilt_root: f64,
        right_endpoint: bool,
    ) -> Result<FactorKey<ImuKinematics>> {
        // Put exact right-boundary observations on the interval's own controls
        // so their information is marginalized, rather than dropped, with it.
        let query = if right_endpoint && time > self.origin {
            Time::from_nanos(time.as_nanos() - 1)
        } else {
            time
        };
        let (segment, _) = self.segment_and_tau(query)?;
        let tau = (time.as_nanos()
            - self.origin.as_nanos()
            - segment * self.parameters.timing.knot_ns()) as f64
            / self.parameters.timing.knot_ns() as f64;
        let controls = self.ensure_segment(segment)?;
        let (biases, bias_tau) = self.ensure_biases(query)?;
        let bias_tau = bias_tau
            + (time.as_nanos() - query.as_nanos()) as f64 / self.parameters.timing.bias_ns() as f64;
        let measured_up = self
            .attitude_at(time)
            .filter(|_| tilt_root != 0.0)
            .map(|a| a.rotation::<Robot>().inverse() * Vector3::<ImuReference, f64>::z_axis());
        Ok(self.graph.add_factor(ImuKinematics {
            controls,
            biases,
            duration: self.parameters.timing.trajectory_spacing.as_secs_f64(),
            gyroscope_information_root: Matrix3::identity() * gyro_root,
            tilt_information_root: Matrix3::identity() * tilt_root,
            tau,
            bias_tau,
            angular_velocity: gyro,
            measured_up,
        })?)
    }

    pub(super) fn retire_preintegration(&mut self, oldest: i64) {
        let first =
            oldest * self.parameters.timing.knot_ns() / self.parameters.timing.interval_ns();
        // Endpoint and boundary factors were removed by marginalization.
        self.preintegration
            .intervals
            .retain(|&index, _| index >= first);
        if self
            .preintegration
            .terminal
            .as_ref()
            .is_some_and(|(segment, _)| *segment < oldest)
        {
            self.preintegration.terminal = None;
        }
        let start =
            Time::from_nanos(self.origin.as_nanos() + oldest * self.parameters.timing.knot_ns());
        let before = self
            .preintegration
            .samples
            .range(..=start)
            .next_back()
            .map_or(start, |(&t, _)| t);
        self.preintegration
            .samples
            .retain(|&time, _| time >= before);
        self.preintegration.dirty.retain(|&index| index >= first);
    }
}
