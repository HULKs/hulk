use coordinate_systems::{Field, ImuReference};
use linear_algebra::{Orientation2, Orientation3, Rotation2};
use nalgebra::UnitComplex;
use ros_z::time::Time;

/// Orientation of the IMU's heading zero in Field, independent of optimized Local.
#[derive(Clone, Copy, Debug)]
pub(crate) struct HeadingReference {
    time: Time,
    imu_to_field: Rotation2<ImuReference, Field, f64>,
    last_tracking_update: Option<Time>,
}

impl HeadingReference {
    pub(crate) fn new(
        time: Time,
        field: Orientation2<Field, f64>,
        imu: Orientation3<ImuReference, f64>,
    ) -> Self {
        Self {
            time,
            imu_to_field: Rotation2::wrap(field.inner * imu_heading(imu).inner.inverse()),
            last_tracking_update: None,
        }
    }

    pub(crate) fn snapshot(&self, max_error: f64) -> types::localization::FieldHeadingReference {
        types::localization::FieldHeadingReference {
            time: self.time,
            imu_to_field: self.imu_to_field,
            max_error,
        }
    }

    pub(crate) fn expected(
        &self,
        imu: Orientation3<ImuReference, f64>,
    ) -> Orientation2<Field, f64> {
        self.imu_to_field * imu_heading(imu)
    }

    pub(crate) fn interrupt_tracking(&mut self) {
        self.last_tracking_update = None;
    }

    /// Called once per fresh accepted tracking image, never for recovery or repeated solves.
    pub(crate) fn observe_tracking(
        &mut self,
        time: Time,
        field: Orientation2<Field, f64>,
        imu: Orientation3<ImuReference, f64>,
        max_drift_per_second: f64,
    ) {
        if let Some(previous) = self.last_tracking_update {
            if time <= previous {
                return;
            }
            let limit = max_drift_per_second * time.duration_since(previous).as_secs_f64();
            let error = self.expected(imu).rotation_to(field).inner.angle();
            self.imu_to_field.inner *= UnitComplex::new(error.clamp(-limit, limit));
        }
        self.last_tracking_update = Some(time);
        self.time = time;
    }
}

fn imu_heading(attitude: Orientation3<ImuReference, f64>) -> Orientation2<ImuReference, f64> {
    Orientation2::new(attitude.euler_angles().2)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn turns_wrap_and_drift_updates_cannot_ratchet_recovery() {
        let imu = |angle: f64| Orientation3::from_euler_angles(0.0, 0.0, angle.to_radians());
        let field = |angle: f64| Orientation2::new(angle.to_radians());
        let mut reference = HeadingReference::new(Time::from_nanos(0), field(170.0), imu(120.0));
        assert!(
            reference
                .expected(imu(-160.0))
                .rotation_to(field(-110.0))
                .inner
                .angle()
                .abs()
                < 1e-12
        );
        let start = Time::from_nanos(1_000_000_000);
        let rate = 0.5_f64.to_radians();
        reference.observe_tracking(start, field(170.0), imu(120.0), rate);
        let later = start + Duration::from_secs(1);
        reference.observe_tracking(later, field(172.0), imu(120.0), rate);
        assert!((reference.expected(imu(120.0)).angle().to_degrees() - 170.5).abs() < 1e-9);
        // Re-solving or receiving an older frame earns no additional drift correction.
        reference.observe_tracking(later, field(-100.0), imu(120.0), rate);
        reference.observe_tracking(start, field(-100.0), imu(120.0), rate);
        assert!((reference.expected(imu(120.0)).angle().to_degrees() - 170.5).abs() < 1e-9);
        reference.interrupt_tracking();
        reference.observe_tracking(
            later + Duration::from_secs(600),
            field(-100.0),
            imu(120.0),
            rate,
        );
        assert!((reference.expected(imu(120.0)).angle().to_degrees() - 170.5).abs() < 1e-9);
    }
}
