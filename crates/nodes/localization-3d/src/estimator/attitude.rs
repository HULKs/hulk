use super::{Estimator, control_keys};
use color_eyre::Result;
use coordinate_systems::ImuReference;
use linear_algebra::Orientation3;
use localization_fagra::factors::RelativeYaw;
use ros_z::time::Time;

impl Estimator {
    /// Boundary attitudes summarize the source-time IMU stream. Relative headings
    /// carry no absolute-yaw anchor. Rebuilding the small active set handles late samples.
    pub(super) fn prepare_attitude(&mut self) -> Result<()> {
        for segment in self.segments() {
            let start = Time::from_nanos(
                self.origin.as_nanos() + segment * self.parameters.timing.knot_ns(),
            );
            let end = Time::from_nanos(start.as_nanos() + self.parameters.timing.knot_ns());
            let Some(a) = self.attitude_at(start) else {
                continue;
            };
            let Some(b) = self.attitude_at(end) else {
                continue;
            };
            if let Some(key) = self.yaw_factors.remove(&segment) {
                self.graph.remove_factor(key)?;
            }
            let key = self.graph.add_factor(RelativeYaw {
                controls: control_keys(&self.controls, segment)?,
                duration: self.parameters.timing.trajectory_spacing.as_secs_f64(),
                end_tau: 1.0,
                measured_yaw_change: yaw_change(a, b),
                information_root: self.parameters.model.relative_yaw_variance.sqrt().recip(),
            })?;
            self.yaw_factors.insert(segment, key);
        }
        if let Some((&time, &orientation)) = self.attitudes.last_key_value() {
            let (segment, tau) = self.segment_and_tau(time)?;
            let start = Time::from_nanos(
                self.origin.as_nanos() + segment * self.parameters.timing.knot_ns(),
            );
            if tau > 0.0
                && let Some(anchor) = self.attitude_at(start)
            {
                self.current_yaw = Some(self.graph.add_factor(RelativeYaw {
                    controls: control_keys(&self.controls, segment)?,
                    duration: self.parameters.timing.trajectory_spacing.as_secs_f64(),
                    end_tau: tau,
                    measured_yaw_change: yaw_change(anchor, orientation),
                    information_root: self.parameters.model.relative_yaw_variance.sqrt().recip(),
                })?);
            }
        }
        Ok(())
    }

    pub(crate) fn attitude_at(&self, time: Time) -> Option<Orientation3<ImuReference, f64>> {
        let (&before, a) = self.attitudes.range(..=time).next_back()?;
        if before == time {
            return Some(*a);
        }
        let (&after, b) = self.attitudes.range(time..).next()?;
        if after.duration_since(before) > self.parameters.timing.max_imu_gap {
            return None;
        }
        let fraction = (time.as_nanos() - before.as_nanos()) as f64
            / (after.as_nanos() - before.as_nanos()) as f64;
        Some(a.slerp(*b, fraction))
    }
}

fn yaw_change(a: Orientation3<ImuReference, f64>, b: Orientation3<ImuReference, f64>) -> f64 {
    let a = a.inner * nalgebra::Vector3::x();
    let b = b.inner * nalgebra::Vector3::x();
    let difference = b.y.atan2(b.x) - a.y.atan2(a.x);
    difference.sin().atan2(difference.cos())
}
