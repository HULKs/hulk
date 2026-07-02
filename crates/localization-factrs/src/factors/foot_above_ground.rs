use std::time::SystemTime;

use factrs::{
    linalg::{ForwardProp, Numeric, VectorX},
    traits::Residual,
    variables::{MatrixLieGroup, SE23},
};
use nalgebra::Point3;

use crate::{
    SE23Spline,
    utils::{interval_dt, tau},
};

#[derive(Debug, Clone)]
pub struct FootHeightMeasurement {
    pub time: SystemTime,
    pub left_sole_in_robot: Point3<f64>,
    pub right_sole_in_robot: Point3<f64>,
}

#[derive(Debug, Clone)]
pub struct IntervalFootAboveGroundFactor {
    measurements: Vec<FootHeightMeasurement>,
    measurement_taus: Vec<f64>,
    start_time: SystemTime,
    end_time: SystemTime,
    duration: f64,
    sigma: f64,
}

#[factrs::mark]
impl Residual for IntervalFootAboveGroundFactor {
    type Input = (SE23, SE23);
    type Differ = ForwardProp;

    fn dim_out(&self) -> usize {
        self.measurements.len() * 2
    }

    fn residual<T: Numeric>(&self, (start, end): (SE23<T>, SE23<T>)) -> VectorX<T> {
        self.residuals_on_spline(start, end)
    }
}

impl IntervalFootAboveGroundFactor {
    pub fn new(
        measurements: Vec<FootHeightMeasurement>,
        start_time: SystemTime,
        end_time: SystemTime,
        sigma: f64,
    ) -> Self {
        assert!(sigma > 0.0, "foot ground sigma must be positive");

        let duration = interval_dt::<f64>(start_time, end_time);
        let measurement_taus = measurements
            .iter()
            .map(|measurement| tau::<f64>(start_time, end_time, measurement.time))
            .collect();

        Self {
            measurements,
            measurement_taus,
            start_time,
            end_time,
            duration,
            sigma,
        }
    }

    pub fn extend_measurements(
        &mut self,
        measurements: impl IntoIterator<Item = FootHeightMeasurement>,
    ) {
        for measurement in measurements {
            self.measurement_taus.push(tau::<f64>(
                self.start_time,
                self.end_time,
                measurement.time,
            ));
            self.measurements.push(measurement);
        }
    }

    fn residuals_on_spline<T: Numeric>(&self, start: SE23<T>, end: SE23<T>) -> VectorX<T> {
        assert_eq!(self.measurements.len(), self.measurement_taus.len());

        let spline = SE23Spline::new(start, end, T::from(self.duration));
        let mut residuals = VectorX::<T>::zeros(self.dim_out());

        for (index, (measurement, measurement_tau)) in self
            .measurements
            .iter()
            .zip(self.measurement_taus.iter())
            .enumerate()
        {
            let pose = spline.evaluate(T::from(*measurement_tau));
            residuals[index * 2] = foot_residual(&pose, &measurement.left_sole_in_robot, self);
            residuals[index * 2 + 1] = foot_residual(&pose, &measurement.right_sole_in_robot, self);
        }

        residuals
    }
}

fn foot_residual<T: Numeric>(
    pose: &SE23<T>,
    sole_in_robot: &Point3<f64>,
    factor: &IntervalFootAboveGroundFactor,
) -> T {
    let sole_in_field = pose.rot().apply(sole_in_robot.coords.cast::<T>().as_view()) + pose.xyz();
    if sole_in_field.z < T::zero() {
        -sole_in_field.z / T::from(factor.sigma)
    } else {
        T::zero()
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use factrs::{
        core::{SO3, Vector3},
        traits::Variable,
        variables::SE23,
    };
    use nalgebra::{Point3, vector};

    use super::*;

    fn measurement(time: SystemTime, left_z: f64, right_z: f64) -> FootHeightMeasurement {
        FootHeightMeasurement {
            time,
            left_sole_in_robot: Point3::new(0.0, 0.05, left_z),
            right_sole_in_robot: Point3::new(0.0, -0.05, right_z),
        }
    }

    fn state(position: Vector3) -> SE23 {
        SE23::from_rot_vel_trans(SO3::identity(), Vector3::zeros(), position)
    }

    #[test]
    fn residual_is_near_zero_when_both_feet_are_above_ground() {
        let start_time = SystemTime::UNIX_EPOCH;
        let factor = IntervalFootAboveGroundFactor::new(
            vec![measurement(start_time, 0.0, -0.1)],
            start_time,
            start_time + Duration::from_secs(1),
            0.01,
        );

        let residual = factor
            .residuals_on_spline(state(vector![0.0, 0.0, 0.2]), state(vector![0.0, 0.0, 0.2]));

        assert!(residual.iter().all(|value| *value < 1.0e-9));
    }

    #[test]
    fn residual_penalizes_left_foot_below_ground() {
        let start_time = SystemTime::UNIX_EPOCH;
        let factor = IntervalFootAboveGroundFactor::new(
            vec![measurement(start_time, -0.2, 0.1)],
            start_time,
            start_time + Duration::from_secs(1),
            0.01,
        );

        let residual = factor.residuals_on_spline(state(Vector3::zeros()), state(Vector3::zeros()));

        assert!(residual[0] > 10.0, "residual was {residual:?}");
        assert!(residual[1] < 1.0e-9, "residual was {residual:?}");
    }

    #[test]
    fn residual_penalizes_right_foot_below_ground() {
        let start_time = SystemTime::UNIX_EPOCH;
        let factor = IntervalFootAboveGroundFactor::new(
            vec![measurement(start_time, 0.1, -0.2)],
            start_time,
            start_time + Duration::from_secs(1),
            0.01,
        );

        let residual = factor.residuals_on_spline(state(Vector3::zeros()), state(Vector3::zeros()));

        assert!(residual[0] < 1.0e-9, "residual was {residual:?}");
        assert!(residual[1] > 10.0, "residual was {residual:?}");
    }

    #[test]
    fn residual_uses_pose_translation() {
        let start_time = SystemTime::UNIX_EPOCH;
        let factor = IntervalFootAboveGroundFactor::new(
            vec![measurement(start_time, -0.1, -0.1)],
            start_time,
            start_time + Duration::from_secs(1),
            0.01,
        );

        let residual = factor
            .residuals_on_spline(state(vector![0.0, 0.0, 0.2]), state(vector![0.0, 0.0, 0.2]));

        assert!(residual.iter().all(|value| *value < 1.0e-9));
    }
}
