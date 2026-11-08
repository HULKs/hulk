//! Brings ground data of different times into the current ground frame.
//!
//! Ground moves with the robot, and producers hold, delay and publish their data at their own
//! rates, so the latest messages of two topics generally do not refer to the same ground frame.
//! Each message tells the time of its ground frame by its source time, and odometry relates the
//! ground frames of all times. Consumers keep a snapshot as it was published and move it into the
//! newest frame here whenever they use it; data whose ground frame is unknown is left out.

use std::{num::NonZeroUsize, ops::Mul, time::Duration};

use coordinate_systems::{Field, Ground, Odometry};
use linear_algebra::{Isometry2, Pose2};
use ros_z::time::Time;

use crate::odometry_history::OdometryHistory;

/// How many poses to queue in a consumer between its updates; half a second at 500 Hz.
pub const ODOMETRY_QUEUE_CAPACITY: NonZeroUsize = NonZeroUsize::new(256).unwrap();

/// How long poses are kept, which is the oldest ground data that can still be used.
const RETENTION: Duration = Duration::from_secs(2);
/// Poses are 2 ms apart; beyond this distance, the robot could have moved unnoticed.
const MAXIMUM_POSE_DISTANCE: Duration = Duration::from_millis(10);
/// How long the newest pose may stay the newest one before it no longer counts as current.
const MAXIMUM_ODOMETRY_SILENCE: Duration = Duration::from_millis(500);

/// The relation of the ground frames of all recent times to each other and to the field.
#[derive(Clone, Debug)]
pub struct GroundAlignment {
    history: OdometryHistory,
    /// The newest pose time, and when it became the newest one by the consumer's clock.
    ///
    /// This is deliberately not a comparison of the consumer's clock with the time of the pose,
    /// which comes from another clock on the robot.
    newest_pose: Option<(Time, Time)>,
}

impl Default for GroundAlignment {
    fn default() -> Self {
        Self {
            history: OdometryHistory::new(RETENTION),
            newest_pose: None,
        }
    }
}

impl GroundAlignment {
    /// Appends the odometry pose at `time`; poses must come in time order, others are ignored.
    pub fn push_pose(&mut self, time: Time, pose: Pose2<Odometry>) {
        self.history.push_pose(time, pose);
    }

    /// Records a ground-to-field estimate referring to `time`, the time of its image.
    pub fn push_ground_to_field(&mut self, time: Time, ground_to_field: Isometry2<Ground, Field>) {
        self.history.push_ground_to_field(time, ground_to_field);
    }

    /// Starts over if odometry has been silent for too long; call this regularly.
    ///
    /// Odometry does not account for the motion it missed, so the current frame and the relation
    /// to the field are unknown until odometry and localization have recovered. Meanwhile, nobody
    /// should plan with a frozen pose.
    pub fn expire_if_silent(&mut self, now: Time) {
        let newest_pose_time = self.history.latest_pose().map(|(time, _)| time);
        match (self.newest_pose, newest_pose_time) {
            (Some((known_time, since)), Some(time)) if known_time == time => {
                if now.duration_since(since) > MAXIMUM_ODOMETRY_SILENCE {
                    self.history.clear();
                    self.newest_pose = None;
                }
            }
            (_, time) => self.newest_pose = time.map(|time| (time, now)),
        }
    }

    /// The time of the current ground frame and where that frame is in the field.
    pub fn current_frame(&self) -> Option<(Time, Isometry2<Ground, Field>)> {
        let (time, pose) = self.history.latest_pose()?;
        Some((time, self.history.ground_to_field_at(pose)?))
    }

    /// Where the current ground frame is in the field.
    pub fn current_ground_to_field(&self) -> Option<Isometry2<Ground, Field>> {
        self.current_frame()
            .map(|(_, ground_to_field)| ground_to_field)
    }

    /// Where the ground frame of `time` is in the field.
    pub fn ground_to_field_at(&self, time: Time) -> Option<Isometry2<Ground, Field>> {
        self.history
            .ground_to_field_at_time(time, MAXIMUM_POSE_DISTANCE)
    }

    /// Where the ground frame of `time` is in the current ground frame.
    pub fn ground_to_current_ground(&self, time: Time) -> Option<Isometry2<Ground, Ground>> {
        self.history
            .ground_to_current_ground(time, MAXIMUM_POSE_DISTANCE)
    }

    /// Moves `data` from the ground frame of `time` into the current one.
    pub fn in_current_ground<T>(&self, time: Time, data: T) -> Option<T>
    where
        Isometry2<Ground, Ground>: Mul<T, Output = T>,
    {
        Some(self.ground_to_current_ground(time)? * data)
    }
}

#[cfg(test)]
mod tests {
    use std::f32::consts::FRAC_PI_2;

    use approx::assert_relative_eq;
    use linear_algebra::{point, vector};
    use types::obstacles::Obstacle;

    use super::*;

    fn time(milliseconds: i64) -> Time {
        Time::from_nanos((1_000 + milliseconds) * 1_000_000)
    }

    fn walking() -> GroundAlignment {
        let mut alignment = GroundAlignment::default();
        // Localization of the first pose arrives once the robot has walked and turned.
        for milliseconds in (0..=100).step_by(2) {
            let fraction = milliseconds as f32 / 100.0;
            alignment.push_pose(
                time(milliseconds),
                Pose2::new(point![fraction, 0.0], fraction * FRAC_PI_2),
            );
        }
        alignment.push_ground_to_field(time(0), Isometry2::from_parts(vector![1.0, 2.0], 0.0));
        alignment
    }

    #[test]
    fn current_frame_follows_odometry_from_a_late_localization() {
        let alignment = walking();

        let (frame_time, ground_to_field) = alignment.current_frame().unwrap();

        assert_eq!(frame_time, time(100));
        assert_relative_eq!(ground_to_field.as_pose().position(), point![2.0, 2.0]);
        assert_relative_eq!(ground_to_field.as_pose().angle(), FRAC_PI_2, epsilon = 1e-5);
    }

    /// A snapshot and the current transform agree on the field, whichever frame it is taken in.
    #[test]
    fn data_of_an_old_frame_stays_in_place_in_the_field() {
        let alignment = walking();
        let in_old_ground = point![<Ground>, 2.0, 0.0];
        let in_field = alignment.ground_to_field_at(time(0)).unwrap() * in_old_ground;
        assert_relative_eq!(in_field, point![3.0, 2.0]);

        let moved = alignment
            .in_current_ground(time(0), Obstacle::robot(in_old_ground, 0.2, 0.25))
            .unwrap();

        assert_relative_eq!(moved.position, point![0.0, -1.0], epsilon = 1e-5);
        assert_relative_eq!(
            alignment.current_ground_to_field().unwrap() * moved.position,
            in_field,
            epsilon = 1e-5
        );
    }

    #[test]
    fn data_without_a_known_frame_is_left_out() {
        let alignment = walking();

        // Too far ahead of the newest pose.
        assert!(alignment.ground_to_current_ground(time(111)).is_none());
        // Older than the retention.
        let mut alignment = alignment;
        alignment.push_pose(time(3_000), Pose2::zero());
        assert!(alignment.ground_to_current_ground(time(50)).is_none());
    }

    #[test]
    fn current_frame_needs_localization() {
        let mut alignment = GroundAlignment::default();
        alignment.push_pose(time(0), Pose2::zero());

        assert!(alignment.current_frame().is_none());
        assert!(alignment.ground_to_field_at(time(0)).is_none());
        assert!(alignment.ground_to_current_ground(time(0)).is_some());
    }

    #[test]
    fn silent_odometry_ends_the_current_frame_until_everything_recovers() {
        let mut alignment = walking();
        // The clock of the consumer is unrelated to the time of the poses.
        let now = |milliseconds: i64| Time::from_nanos((5_000_000 + milliseconds) * 1_000_000);

        alignment.expire_if_silent(now(0));
        alignment.expire_if_silent(now(500));
        assert!(alignment.current_frame().is_some());

        alignment.expire_if_silent(now(501));
        assert!(alignment.current_frame().is_none());
        assert!(alignment.ground_to_current_ground(time(100)).is_none());

        alignment.push_pose(time(800), Pose2::zero());
        alignment.expire_if_silent(now(800));
        assert!(alignment.current_frame().is_none());

        alignment.push_ground_to_field(time(800), Isometry2::identity());
        alignment.push_pose(time(802), Pose2::zero());
        alignment.expire_if_silent(now(802));
        assert!(alignment.current_frame().is_some());
    }

    #[test]
    fn regular_poses_keep_the_current_frame() {
        let mut alignment = walking();

        for step in 0..1_000 {
            alignment.push_pose(time(102 + step * 2), Pose2::zero());
            alignment.expire_if_silent(time(step * 2));
        }

        assert!(alignment.current_frame().is_some());
    }
}
