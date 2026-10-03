//! IMU preintegration on SO(3), following GTSAM 4.2 ManifoldPreintegration
//! and ImuFactor's continuous-time noise propagation (BSD-licensed reference).
//! Unlike GTSAM's NavState chart, our error order is [rotation, velocity, position],
//! with a right rotation perturbation and both vectors in the interval-start axes.
//! See docs/tooling/localization_preintegration.md for frame/covariance mapping.

use coordinate_systems::Robot;
use fagra::{EvaluationError, Variable};
use linear_algebra::{Rotation3, Vector3};
use nalgebra::{Matrix3, RealField, SMatrix};

use crate::{
    finite,
    variables::{ImuBias, rotation},
};

pub type Matrix9<R = f64> = SMatrix<R, 9, 9>;
pub type BiasJacobian<R = f64> = SMatrix<R, 9, 6>;

#[derive(Clone, Copy, Debug)]
pub struct ImuNoise {
    /// Continuous-time noise variances: density squared, not per-sample variance.
    pub gyroscope: f64,
    pub accelerometer: f64,
    pub integration: f64,
}

#[derive(Clone, Debug)]
pub struct ImuDelta<R: RealField + Copy = f64> {
    pub duration: R,
    /// End Robot axes into start Robot axes.
    pub rotation: Rotation3<Robot, Robot, R>,
    /// Integrated specific force, in start Robot axes. Gravity is added by the factor.
    pub velocity: Vector3<Robot, R>,
    pub position: Vector3<Robot, R>,
    pub reference_biases: [ImuBias<R>; 2],
    /// Row order R,V,P; column order gyro,accel for each coarse bias knot.
    pub bias_jacobians: [BiasJacobian<R>; 2],
}

pub(crate) struct CorrectedImuDelta<R: RealField + Copy> {
    pub rotation: Rotation3<Robot, Robot, R>,
    pub velocity: Vector3<Robot, R>,
    pub position: Vector3<Robot, R>,
    pub rotation_jacobian: Matrix3<R>,
}

impl<R: RealField + Copy> ImuDelta<R> {
    pub(crate) fn corrected(
        &self,
        biases: [&ImuBias<R>; 2],
    ) -> Result<CorrectedImuDelta<R>, EvaluationError> {
        let correction = self.bias_jacobians[0]
            * (biases[0].log() - self.reference_biases[0].log())
            + self.bias_jacobians[1] * (biases[1].log() - self.reference_biases[1].log());
        finite(correction.iter())?;
        let angle = correction.fixed_rows::<3>(0).into_owned();
        Ok(CorrectedImuDelta {
            rotation: self.rotation * Rotation3::wrap(rotation::exp(angle)),
            velocity: self.velocity + Vector3::wrap(correction.fixed_rows::<3>(3).into_owned()),
            position: self.position + Vector3::wrap(correction.fixed_rows::<3>(6).into_owned()),
            rotation_jacobian: rotation::right_jacobian(angle),
        })
    }
}

/// All buffers are fixed-size. Samples are integrated once on the normal append
/// path; the estimator retains raw data only for late-data/bias-reference rebuilds.
#[derive(Clone, Debug)]
pub struct ImuPreintegrator {
    pub delta: ImuDelta,
    pub covariance: Matrix9,
    pub acceleration: bool,
}

impl ImuPreintegrator {
    pub fn new(reference_biases: [ImuBias; 2], acceleration: bool) -> Self {
        Self {
            delta: ImuDelta {
                duration: 0.0,
                rotation: Rotation3::wrap(nalgebra::UnitQuaternion::identity()),
                velocity: Vector3::zeros(),
                position: Vector3::zeros(),
                reference_biases,
                bias_jacobians: [BiasJacobian::zeros(); 2],
            },
            covariance: Matrix9::zeros(),
            acceleration,
        }
    }

    /// Zero-order-held calibrated Robot-axis readings over dt. `bias_tau` locates
    /// this integration step on the independent coarse linear-bias interval.
    /// Integrate at the physical IMU origin; the factor handles the mounting offset.
    pub fn integrate(
        &mut self,
        gyro: Vector3<Robot, f64>,
        force: Option<Vector3<Robot, f64>>,
        dt: f64,
        bias_tau: f64,
        noise: ImuNoise,
    ) -> Result<(), EvaluationError> {
        if !dt.is_finite()
            || dt <= 0.0
            || !(0.0..=1.0).contains(&bias_tau)
            || !noise.gyroscope.is_finite()
            || noise.gyroscope <= 0.0
            || (self.acceleration
                && (force.is_none()
                    || !noise.accelerometer.is_finite()
                    || noise.accelerometer <= 0.0
                    || !noise.integration.is_finite()
                    || noise.integration <= 0.0))
        {
            return Err(EvaluationError::InvalidEvaluation);
        }
        finite(gyro.inner.iter())?;
        for bias in &self.delta.reference_biases {
            finite(
                bias.gyroscope
                    .inner
                    .iter()
                    .chain(bias.accelerometer.inner.iter()),
            )?;
        }
        let weights = [1.0 - bias_tau, bias_tau];
        let omega = gyro.inner
            - self.delta.reference_biases[0].gyroscope.inner * weights[0]
            - self.delta.reference_biases[1].gyroscope.inner * weights[1];
        let angle = omega * dt;
        finite(angle.iter())?;
        let increment = rotation::exp(angle);
        let d = increment.to_rotation_matrix().inverse().into_inner();
        let jr = rotation::right_jacobian(angle);
        let old_rotation = self.delta.rotation.inner.to_rotation_matrix().into_inner();
        let mut coupling = Matrix3::zeros();
        if self.acceleration {
            let acceleration = force.ok_or(EvaluationError::InvalidEvaluation)?.inner
                - self.delta.reference_biases[0].accelerometer.inner * weights[0]
                - self.delta.reference_biases[1].accelerometer.inner * weights[1];
            finite(acceleration.iter())?;
            let a = old_rotation * acceleration;
            self.delta.position.inner += self.delta.velocity.inner * dt + a * (0.5 * dt * dt);
            self.delta.velocity.inner += a * dt;
            coupling = -old_rotation * acceleration.cross_matrix();
        }
        // Propagate bias sensitivities for both linearly interpolated knots.
        for (i, weight) in weights.into_iter().enumerate() {
            let j = &mut self.delta.bias_jacobians[i];
            let old_r = j.fixed_rows::<3>(0).into_owned();
            if self.acceleration {
                let old_v = j.fixed_rows::<3>(3).into_owned();
                let mut change = coupling * old_r;
                let acceleration_bias =
                    change.fixed_columns::<3>(3).into_owned() - old_rotation * weight;
                change
                    .fixed_columns_mut::<3>(3)
                    .copy_from(&acceleration_bias);
                let p = j.fixed_rows::<3>(6).into_owned() + old_v * dt + change * (0.5 * dt * dt);
                j.fixed_rows_mut::<3>(6).copy_from(&p);
                j.fixed_rows_mut::<3>(3).copy_from(&(old_v + change * dt));
            }
            let mut r = d * old_r;
            let block = r.fixed_columns::<3>(0).into_owned() - jr * (dt * weight);
            r.fixed_columns_mut::<3>(0).copy_from(&block);
            j.fixed_rows_mut::<3>(0).copy_from(&r);
        }
        if self.acceleration {
            // F = [D,0,0; E,I,0; H,dt*I,I]. Use its block structure rather
            // than two general 9x9 products at sensor rate.
            let e = coupling * dt;
            let h = coupling * (0.5 * dt * dt);
            let mut left = Matrix9::zeros();
            for col in [0, 3, 6] {
                let r = self.covariance.fixed_view::<3, 3>(0, col);
                let v = self.covariance.fixed_view::<3, 3>(3, col);
                let p = self.covariance.fixed_view::<3, 3>(6, col);
                left.fixed_view_mut::<3, 3>(0, col).copy_from(&(d * r));
                left.fixed_view_mut::<3, 3>(3, col).copy_from(&(e * r + v));
                left.fixed_view_mut::<3, 3>(6, col)
                    .copy_from(&(h * r + v * dt + p));
            }
            for row in [0, 3, 6] {
                let r = left.fixed_view::<3, 3>(row, 0);
                let v = left.fixed_view::<3, 3>(row, 3);
                let p = left.fixed_view::<3, 3>(row, 6);
                self.covariance
                    .fixed_view_mut::<3, 3>(row, 0)
                    .copy_from(&(r * d.transpose()));
                self.covariance
                    .fixed_view_mut::<3, 3>(row, 3)
                    .copy_from(&(r * e.transpose() + v));
                self.covariance
                    .fixed_view_mut::<3, 3>(row, 6)
                    .copy_from(&(r * h.transpose() + v * dt + p));
            }
            // B (Qa/dt) Bᵀ. Isotropic force noise is unchanged by old_rotation.
            for axis in 0..3 {
                self.covariance[(3 + axis, 3 + axis)] += noise.accelerometer * dt;
                self.covariance[(3 + axis, 6 + axis)] += noise.accelerometer * 0.5 * dt * dt;
                self.covariance[(6 + axis, 3 + axis)] += noise.accelerometer * 0.5 * dt * dt;
                self.covariance[(6 + axis, 6 + axis)] +=
                    noise.accelerometer * 0.25 * dt * dt * dt + noise.integration * dt;
            }
        } else {
            let rr = d * self.covariance.fixed_view::<3, 3>(0, 0) * d.transpose();
            self.covariance.fixed_view_mut::<3, 3>(0, 0).copy_from(&rr);
        }
        let rr = self.covariance.fixed_view::<3, 3>(0, 0).into_owned()
            + jr * jr.transpose() * (noise.gyroscope * dt);
        self.covariance.fixed_view_mut::<3, 3>(0, 0).copy_from(&rr);
        self.delta.rotation = self.delta.rotation * Rotation3::wrap(increment);
        self.delta.duration += dt;
        finite(
            self.covariance
                .iter()
                .chain(self.delta.velocity.inner.iter())
                .chain(self.delta.position.inner.iter()),
        )?;
        Ok(())
    }

    /// Whiten once when publishing an interval, not at every sensor step.
    pub fn information(&self) -> Result<PreintegrationInformation, EvaluationError> {
        if self.delta.duration <= 0.0 {
            return Err(EvaluationError::InvalidEvaluation);
        }
        if self.acceleration {
            let covariance = (self.covariance + self.covariance.transpose()) * 0.5;
            let root = covariance
                .cholesky()
                .and_then(|l| l.l().try_inverse())
                .ok_or(EvaluationError::InvalidEvaluation)?;
            finite(root.iter())?;
            Ok(PreintegrationInformation::Full(root))
        } else {
            let covariance = self.covariance.fixed_view::<3, 3>(0, 0).into_owned();
            let root = covariance
                .cholesky()
                .and_then(|l| l.l().try_inverse())
                .ok_or(EvaluationError::InvalidEvaluation)?;
            finite(root.iter())?;
            Ok(PreintegrationInformation::Rotation(root))
        }
    }
}

#[derive(Clone, Debug)]
pub enum PreintegrationInformation<R: RealField + Copy = f64> {
    Rotation(Matrix3<R>),
    Full(Matrix9<R>),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn variable_step_turn_and_impulse_match_gtsam_reference() {
        let bias = ImuBias {
            gyroscope: Vector3::wrap(nalgebra::vector![0.02, -0.01, 0.03]),
            accelerometer: Vector3::wrap(nalgebra::vector![0.1, -0.2, 0.05]),
        };
        let mut p = ImuPreintegrator::new([bias.clone(), bias], true);
        let mut t = 0.0;
        for i in 0..40 {
            let dt = [0.001, 0.002, 0.003, 0.004][i % 4];
            p.integrate(
                Vector3::wrap(nalgebra::vector![2.0 + t, -1.0 + 2.0 * t, 3.0 - t]),
                Some(Vector3::wrap(nalgebra::vector![
                    0.5 - t,
                    0.2 + t,
                    9.81 + if i == 17 { 20.0 } else { 0.0 }
                ])),
                dt,
                (t + dt * 0.5) / 5.0,
                ImuNoise {
                    gyroscope: 2e-5,
                    accelerometer: 0.09,
                    integration: 1e-8,
                },
            )
            .unwrap();
            t += dt;
        }
        // Generated by docs/tooling/preintegration_reference.py using GTSAM 4.2.
        let reference: Vec<f64> = include_str!("preintegration_reference.txt")
            .split_whitespace()
            .map(|s| s.parse().unwrap())
            .collect();
        assert_eq!(reference.len(), 90);
        let angle = rotation::log(&p.delta.rotation.inner);
        let mean = angle
            .iter()
            .chain(p.delta.velocity.inner.iter())
            .chain(p.delta.position.inner.iter());
        for (actual, expected) in mean.zip(&reference) {
            assert!((actual - expected).abs() < 1e-12);
        }
        let covariance = Matrix9::from_row_slice(&reference[9..]);
        let PreintegrationInformation::Full(root) = p.information().unwrap() else {
            unreachable!()
        };
        assert!((root * (p.covariance - covariance) * root.transpose()).amax() < 1e-7);
    }

    fn integrate(biases: [ImuBias; 2]) -> ImuPreintegrator {
        let mut p = ImuPreintegrator::new(biases, true);
        for i in 0..50 {
            let t = i as f64 * 0.002;
            p.integrate(
                Vector3::wrap(nalgebra::vector![0.2 + t, -0.1 + 2.0 * t, 0.3 - t]),
                Some(Vector3::wrap(nalgebra::vector![0.5 - t, 0.2 + t, 9.81])),
                0.002,
                0.2 + t / 5.0,
                ImuNoise {
                    gyroscope: 2e-5,
                    accelerometer: 0.09,
                    integration: 1e-8,
                },
            )
            .unwrap();
        }
        p
    }

    #[test]
    fn interpolated_bias_jacobians_match_reintegration() {
        let biases = [ImuBias::identity(), ImuBias::identity()];
        let base = integrate(biases.clone());
        let h = 1e-6;
        for knot in 0..2 {
            for col in 0..6 {
                let mut increment = nalgebra::SVector::<f64, 6>::zeros();
                increment[col] = h;
                let mut plus = biases.clone();
                plus[knot] = plus[knot].retract(&increment);
                let mut minus = biases.clone();
                minus[knot] = minus[knot].retract(&-increment);
                let a = integrate(plus);
                let b = integrate(minus);
                let mut actual = nalgebra::SVector::<f64, 9>::zeros();
                actual.fixed_rows_mut::<3>(0).copy_from(
                    &((rotation::log(&(base.delta.rotation.inverse() * a.delta.rotation).inner)
                        - rotation::log(
                            &(base.delta.rotation.inverse() * b.delta.rotation).inner,
                        ))
                        / (2.0 * h)),
                );
                actual
                    .fixed_rows_mut::<3>(3)
                    .copy_from(&((a.delta.velocity.inner - b.delta.velocity.inner) / (2.0 * h)));
                actual
                    .fixed_rows_mut::<3>(6)
                    .copy_from(&((a.delta.position.inner - b.delta.position.inner) / (2.0 * h)));
                assert!(
                    (actual - base.delta.bias_jacobians[knot].column(col)).amax() < 1e-8,
                    "knot {knot}, col {col}"
                );
            }
        }
        let root = match base.information().unwrap() {
            PreintegrationInformation::Full(root) => root,
            _ => unreachable!(),
        };
        assert!((root * base.covariance * root.transpose() - Matrix9::identity()).amax() < 1e-10);
    }

    #[test]
    fn stationary_specific_force_and_free_fall_remain_distinct() {
        for force in [0.0, 9.81] {
            let mut p = ImuPreintegrator::new([ImuBias::identity(), ImuBias::identity()], true);
            for i in 0..50 {
                p.integrate(
                    Vector3::zeros(),
                    Some(Vector3::wrap(nalgebra::vector![0.0, 0.0, force])),
                    0.002,
                    i as f64 * 0.002 / 5.0,
                    ImuNoise {
                        gyroscope: 2e-5,
                        accelerometer: 0.09,
                        integration: 1e-8,
                    },
                )
                .unwrap();
            }
            assert!((p.delta.velocity.z() - force * 0.1).abs() < 1e-12);
            assert!((p.delta.position.z() - force * 0.005).abs() < 1e-12);
        }
        let mut p = ImuPreintegrator::new([ImuBias::identity(), ImuBias::identity()], false);
        assert!(
            p.integrate(
                Vector3::zeros(),
                None,
                0.0,
                0.0,
                ImuNoise {
                    gyroscope: 1.0,
                    accelerometer: 0.0,
                    integration: 0.0
                }
            )
            .is_err()
        );
    }
}
