use super::Estimator;
use color_eyre::{Result, eyre::eyre};
use fagra::{StateKey, Variable};
use localization_fagra::{
    factors::{ImuBiasPrior, ImuBiasWalk},
    variables::ImuBias,
};
use nalgebra::SMatrix;
use ros_z::time::Time;

pub(super) fn bias_segment_and_tau(origin: Time, time: Time, spacing_ns: i64) -> (i64, f64) {
    let elapsed = time.as_nanos() - origin.as_nanos();
    (
        elapsed.div_euclid(spacing_ns),
        elapsed.rem_euclid(spacing_ns) as f64 / spacing_ns as f64,
    )
}

fn root(gyro_sigma: f64, accel_sigma: f64) -> SMatrix<f64, 6, 6> {
    SMatrix::from_diagonal(&nalgebra::SVector::<f64, 6>::from_fn(|i, _| {
        if i < 3 {
            gyro_sigma.recip()
        } else {
            accel_sigma.recip()
        }
    }))
}

impl Estimator {
    pub(super) fn initialize_biases(&mut self, time: Time, guess: ImuBias) -> Result<()> {
        let (index, _) = bias_segment_and_tau(self.origin, time, self.parameters.timing.bias_ns());
        let key = self.graph.add(guess);
        self.biases.insert(index, key);
        // Recovery resets confidence, not the initial guess. Never feed a tight
        // posterior back as a prior alongside the same replayed observations.
        // ponytail: recovery relearns confidence; retain a bias-only prior from
        // retired evidence if repeated recovery needs faster calibration reacquisition.
        let elapsed = (time.as_nanos() - self.origin.as_nanos()) as f64 * 1e-9;
        let p = &self.parameters.imu_bias;
        let gyro =
            (p.gyroscope_initial_sigma.powi(2) + elapsed * p.gyroscope_random_walk.powi(2)).sqrt();
        let accel = (p.accelerometer_initial_sigma.powi(2)
            + elapsed * p.accelerometer_random_walk.powi(2))
        .sqrt();
        self.graph.add_factor(ImuBiasPrior {
            bias: key,
            reference: ImuBias::identity(),
            information_root: root(gyro, accel),
        })?;
        self.ensure_biases(time)?;
        Ok(())
    }

    pub(super) fn ensure_biases(&mut self, time: Time) -> Result<([StateKey<ImuBias>; 2], f64)> {
        let (index, tau) =
            bias_segment_and_tau(self.origin, time, self.parameters.timing.bias_ns());
        let (&last, &last_key) = self
            .biases
            .last_key_value()
            .ok_or_else(|| eyre!("missing IMU bias anchor"))?;
        let mut previous = last_key;
        for next in last + 1..=index + 1 {
            let key = self.graph.add(self.graph.get(previous)?.clone());
            self.biases.insert(next, key);
            let seconds = self.parameters.timing.bias_spacing.as_secs_f64();
            let p = &self.parameters.imu_bias;
            self.graph.add_factor(ImuBiasWalk {
                biases: [previous, key],
                information_root: root(
                    p.gyroscope_random_walk * seconds.sqrt(),
                    p.accelerometer_random_walk * seconds.sqrt(),
                ),
            })?;
            previous = key;
        }
        Ok((
            [
                self.biases
                    .get(&index)
                    .copied()
                    .ok_or_else(|| eyre!("retired IMU bias interval"))?,
                self.biases[&(index + 1)],
            ],
            tau,
        ))
    }

    pub(super) fn bias_at(&self, time: Time) -> Result<ImuBias> {
        let (index, tau) =
            bias_segment_and_tau(self.origin, time, self.parameters.timing.bias_ns());
        let Some((&_, &left)) = self.biases.range(..=index).next_back() else {
            return Ok(self
                .graph
                .get(
                    *self
                        .biases
                        .first_key_value()
                        .ok_or_else(|| eyre!("missing bias"))?
                        .1,
                )?
                .clone());
        };
        let a = self.graph.get(left)?;
        let Some(&right) = self.biases.get(&(index + 1)) else {
            return Ok(a.clone());
        };
        let b = self.graph.get(right)?;
        Ok(ImuBias {
            gyroscope: a.gyroscope * (1.0 - tau) + b.gyroscope * tau,
            accelerometer: a.accelerometer * (1.0 - tau) + b.accelerometer * tau,
        })
    }
}
