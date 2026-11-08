//! Pairs localization estimates with the odometry poses of their times.

use std::{collections::VecDeque, time::Duration};

use coordinate_systems::{Field, Ground, Odometry};
use linear_algebra::{Isometry2, Pose2};
use ros_z::time::Time;

fn wrap_angle(angle: f32) -> f32 {
    angle.sin().atan2(angle.cos())
}

/// How far the odometry pose used to place the detections of an image may lie from its time.
///
/// Odometry normally arrives every 2 ms. After a longer gap the robot may have moved and turned
/// unnoticed, which would move all tracks along, so such images are better left out.
pub const MAXIMUM_IMAGE_POSE_DISTANCE: Duration = Duration::from_millis(10);

/// Recent odometry poses and ground-to-field estimates, each with the time it refers to.
///
/// Ground moves with the robot, so a ground-to-field estimate only holds together with the
/// odometry pose of its own time. Localization refers to the time of its image and arrives late,
/// so combining it with the current pose would turn the field around the robot by however much
/// the robot rotated in the meantime. Paired with the pose of its time, it yields an
/// odometry-to-field transform that stays valid while the robot moves, until localization or
/// odometry drift.
#[derive(Clone, Debug)]
pub struct OdometryHistory {
    poses: VecDeque<(Time, Pose2<Odometry>)>,
    /// Estimates whose odometry pose has not been processed yet, ordered by time.
    pending_ground_to_field: VecDeque<(Time, Isometry2<Ground, Field>)>,
    odometry_to_field: Option<Isometry2<Odometry, Field>>,
    retention: Duration,
}

impl OdometryHistory {
    pub fn new(retention: Duration) -> Self {
        Self {
            poses: VecDeque::new(),
            pending_ground_to_field: VecDeque::new(),
            odometry_to_field: None,
            retention,
        }
    }

    /// Appends the odometry pose at `time`; poses must come in time order, others are ignored.
    pub fn push_pose(&mut self, time: Time, pose: Pose2<Odometry>) {
        if self.poses.back().is_some_and(|(latest, _)| time <= *latest) {
            return;
        }
        self.poses.push_back((time, pose));
        while self
            .poses
            .front()
            .is_some_and(|(stamp, _)| time.duration_since(*stamp) > self.retention)
        {
            self.poses.pop_front();
        }
        self.anchor();
    }

    /// Records a ground-to-field estimate referring to `time`, which may lie ahead of the
    /// odometry processed so far.
    pub fn push_ground_to_field(&mut self, time: Time, ground_to_field: Isometry2<Ground, Field>) {
        let index = self
            .pending_ground_to_field
            .partition_point(|(stamp, _)| *stamp <= time);
        self.pending_ground_to_field
            .insert(index, (time, ground_to_field));
        self.anchor();
    }

    pub fn clear(&mut self) {
        self.poses.clear();
        self.pending_ground_to_field.clear();
        self.odometry_to_field = None;
    }

    /// Anchors the newest pending estimate whose odometry pose is known, dropping older ones.
    fn anchor(&mut self) {
        let Some(latest) = self.poses.back().map(|(time, _)| *time) else {
            return;
        };
        let reached = self
            .pending_ground_to_field
            .partition_point(|(stamp, _)| *stamp <= latest);
        let reached: Vec<_> = self.pending_ground_to_field.drain(..reached).collect();
        let anchored = reached
            .into_iter()
            .rev()
            .find_map(|(time, ground_to_field)| {
                let ground_to_odometry = self
                    .pose_near(time, MAXIMUM_IMAGE_POSE_DISTANCE)?
                    .as_transform::<Ground>();
                Some(ground_to_field * ground_to_odometry.inverse())
            });
        if anchored.is_some() {
            self.odometry_to_field = anchored;
        }
    }

    /// Pose at `time`, as far as the known poses tell it.
    ///
    /// Poses at most `maximum_distance` apart are interpolated. Across a longer gap the robot may
    /// have moved in any way, so only a pose at most `maximum_distance` from `time` is taken as is,
    /// and a time in the middle of the gap has no pose. This also bounds how far `time` may lie
    /// outside of the history, and gives the same result before and after the end of a gap arrives.
    pub fn pose_near(&self, time: Time, maximum_distance: Duration) -> Option<Pose2<Odometry>> {
        let after = self.poses.partition_point(|(stamp, _)| *stamp < time);
        let following = self.poses.get(after).copied();
        let preceding = after.checked_sub(1).map(|before| self.poses[before]);
        if let (Some((before_time, before_pose)), Some((after_time, after_pose))) =
            (preceding, following)
            && after_time.duration_since(before_time) <= maximum_distance
        {
            let fraction = (time.duration_since(before_time).as_secs_f32()
                / after_time.duration_since(before_time).as_secs_f32())
            .clamp(0.0, 1.0);
            let position = before_pose.position()
                + (after_pose.position() - before_pose.position()) * fraction;
            let angle = before_pose.angle()
                + wrap_angle(after_pose.angle() - before_pose.angle()) * fraction;
            return Some(Pose2::new(position, angle));
        }
        [preceding, following]
            .into_iter()
            .flatten()
            .filter(|(stamp, _)| stamp.abs_diff(time) <= maximum_distance)
            .min_by_key(|(stamp, _)| stamp.abs_diff(time))
            .map(|(_, pose)| pose)
    }

    /// The newest known odometry pose.
    pub fn latest_pose(&self) -> Option<(Time, Pose2<Odometry>)> {
        self.poses.back().copied()
    }

    /// Ground-to-field transform of the ground frame at `time`, whose pose is taken as in
    /// [`Self::pose_near`].
    pub fn ground_to_field_at_time(
        &self,
        time: Time,
        maximum_distance: Duration,
    ) -> Option<Isometry2<Ground, Field>> {
        self.ground_to_field_at(self.pose_near(time, maximum_distance)?)
    }

    /// Transform from the ground frame at `time` into the ground frame of the newest pose, whose
    /// pose is taken as in [`Self::pose_near`].
    pub fn ground_to_current_ground(
        &self,
        time: Time,
        maximum_distance: Duration,
    ) -> Option<Isometry2<Ground, Ground>> {
        let (_, current_pose) = self.latest_pose()?;
        let pose = self.pose_near(time, maximum_distance)?;
        Some(current_pose.as_transform::<Ground>().inverse() * pose.as_transform::<Ground>())
    }

    /// Ground-to-field transform of the current ground frame.
    pub fn current_ground_to_field(&self) -> Option<Isometry2<Ground, Field>> {
        let (_, pose) = self.latest_pose()?;
        self.ground_to_field_at(pose)
    }

    /// Odometry-to-field transform from the latest ground-to-field estimate whose odometry pose
    /// is known.
    pub fn odometry_to_field(&self) -> Option<Isometry2<Odometry, Field>> {
        self.odometry_to_field
    }

    /// Ground-to-field transform at the odometry `pose`.
    pub fn ground_to_field_at(&self, pose: Pose2<Odometry>) -> Option<Isometry2<Ground, Field>> {
        Some(self.odometry_to_field()? * pose.as_transform::<Ground>())
    }
}

#[cfg(test)]
mod tests {
    use std::f32::consts::{FRAC_PI_2, PI};

    use approx::assert_relative_eq;
    use linear_algebra::{point, vector};

    use super::*;

    fn time(milliseconds: i64) -> Time {
        Time::from_nanos((1_000 + milliseconds) * 1_000_000)
    }

    #[test]
    fn interpolates_between_poses_across_the_angle_wrap() {
        let mut history = OdometryHistory::new(Duration::from_secs(1));
        history.push_pose(time(0), Pose2::new(point![0.0, 0.0], 3.0));
        history.push_pose(time(10), Pose2::new(point![1.0, 2.0], -3.0));

        let maximum_distance = Duration::from_millis(10);
        let pose = history.pose_near(time(5), maximum_distance).unwrap();

        assert_relative_eq!(pose.position(), point![0.5, 1.0]);
        assert_relative_eq!(wrap_angle(pose.angle()).abs(), PI, epsilon = 0.01);
        assert!(history.pose_near(time(21), maximum_distance).is_none());
        assert!(history.pose_near(time(-11), maximum_distance).is_none());
    }

    #[test]
    fn forgets_poses_older_than_the_retention() {
        let mut history = OdometryHistory::new(Duration::from_millis(100));
        for milliseconds in (0..=300).step_by(10) {
            history.push_pose(time(milliseconds), Pose2::zero());
        }

        let maximum_distance = Duration::from_millis(10);
        assert!(history.pose_near(time(150), maximum_distance).is_none());
        assert!(history.pose_near(time(250), maximum_distance).is_some());
    }

    #[test]
    fn late_localization_is_paired_with_the_pose_of_its_time() {
        let mut history = OdometryHistory::new(Duration::from_secs(1));
        // The robot stands at the odometry origin facing +x, which is the field's +y at (1, 2).
        history.push_pose(time(0), Pose2::zero());
        // It walks 1 m forward and turns left before the localization of the first image arrives.
        history.push_pose(time(100), Pose2::new(point![1.0, 0.0], FRAC_PI_2));
        history.push_ground_to_field(
            time(0),
            Isometry2::<Ground, Field>::from_parts(vector![1.0, 2.0], FRAC_PI_2),
        );

        let ground_to_field = history.current_ground_to_field().unwrap().as_pose();

        assert_relative_eq!(ground_to_field.position(), point![1.0, 3.0], epsilon = 1e-5);
        assert_relative_eq!(
            wrap_angle(ground_to_field.angle()).abs(),
            PI,
            epsilon = 1e-5
        );
    }

    #[test]
    fn anchored_localization_outlives_the_retention() {
        let mut history = OdometryHistory::new(Duration::from_millis(100));
        history.push_pose(time(0), Pose2::zero());
        history.push_ground_to_field(time(0), Isometry2::from_parts(vector![1.0, 0.0], 0.0));
        for milliseconds in (10..=1_000).step_by(10) {
            history.push_pose(time(milliseconds), Pose2::zero());
        }

        assert!(history.odometry_to_field().is_some());
    }

    #[test]
    fn transforms_for_a_time_need_a_pose_close_to_it() {
        let mut history = OdometryHistory::new(Duration::from_millis(100));
        history.push_ground_to_field(time(0), Isometry2::identity());
        let maximum_distance = Duration::from_millis(10);
        assert!(history.current_ground_to_field().is_none());

        history.push_pose(time(0), Pose2::zero());
        history.push_pose(time(10), Pose2::new(point![1.0, 0.0], 0.0));

        let at = |milliseconds| {
            history
                .ground_to_field_at_time(time(milliseconds), maximum_distance)
                .map(|ground_to_field| ground_to_field.as_pose().position())
        };
        assert_relative_eq!(at(5).unwrap(), point![0.5, 0.0]);
        assert_relative_eq!(at(20).unwrap(), point![1.0, 0.0]);
        assert!(at(21).is_none());
        assert!(at(-11).is_none());
        assert_relative_eq!(
            history
                .current_ground_to_field()
                .unwrap()
                .as_pose()
                .position(),
            point![1.0, 0.0]
        );
    }

    /// The robot walks and turns during an outage; nothing tells where it was in between.
    #[test]
    fn a_sensor_outage_is_not_bridged() {
        let mut history = OdometryHistory::new(Duration::from_secs(10));
        history.push_ground_to_field(time(0), Isometry2::identity());
        let maximum_distance = Duration::from_millis(10);
        history.push_pose(time(0), Pose2::zero());
        // Before the outage ends, data just after its start is in the frame of the last pose.
        let before_the_end = history.ground_to_field_at_time(time(5), maximum_distance);
        history.push_pose(time(5_000), Pose2::new(point![2.0, 1.0], FRAC_PI_2));

        let position = |milliseconds| {
            history
                .ground_to_field_at_time(time(milliseconds), maximum_distance)
                .map(|ground_to_field| ground_to_field.as_pose().position())
        };
        assert_eq!(
            history
                .ground_to_field_at_time(time(5), maximum_distance)
                .map(|ground_to_field| ground_to_field.as_pose()),
            before_the_end.map(|ground_to_field| ground_to_field.as_pose()),
        );
        assert_relative_eq!(position(0).unwrap(), point![0.0, 0.0]);
        assert_relative_eq!(position(10).unwrap(), point![0.0, 0.0]);
        assert!(position(11).is_none());
        assert!(position(2_500).is_none());
        assert!(position(4_989).is_none());
        assert_relative_eq!(position(4_990).unwrap(), point![2.0, 1.0]);
        assert_relative_eq!(position(5_000).unwrap(), point![2.0, 1.0]);
        assert!(
            history
                .ground_to_current_ground(time(2_500), maximum_distance)
                .is_none()
        );
    }

    /// A localization of an image during an outage has no pose to hold together with.
    #[test]
    fn localization_during_a_sensor_outage_is_not_anchored() {
        let mut history = OdometryHistory::new(Duration::from_secs(10));
        history.push_pose(time(0), Pose2::zero());
        history.push_ground_to_field(time(0), Isometry2::from_parts(vector![1.0, 0.0], 0.0));
        history.push_ground_to_field(time(2_500), Isometry2::from_parts(vector![5.0, 0.0], 0.0));
        history.push_pose(time(5_000), Pose2::zero());

        assert_relative_eq!(
            history.odometry_to_field().unwrap().as_pose().position(),
            point![1.0, 0.0]
        );
    }

    #[test]
    fn ground_data_of_a_time_moves_into_the_current_ground_frame() {
        let mut history = OdometryHistory::new(Duration::from_secs(1));
        let maximum_distance = Duration::from_millis(10);
        history.push_pose(time(0), Pose2::zero());
        // Walks 1 m forward and turns left, so what was 2 m ahead is now 1 m to its right.
        history.push_pose(time(10), Pose2::new(point![1.0, 0.0], FRAC_PI_2));

        let ground_to_current_ground = history
            .ground_to_current_ground(time(0), maximum_distance)
            .unwrap();

        assert_relative_eq!(
            ground_to_current_ground * point![<Ground>, 2.0, 0.0],
            point![0.0, -1.0],
            epsilon = 1e-5
        );
        assert_relative_eq!(
            ground_to_current_ground * vector![<Ground>, 1.0, 0.0],
            vector![0.0, -1.0],
            epsilon = 1e-5
        );
        assert_relative_eq!(
            history
                .ground_to_current_ground(time(10), maximum_distance)
                .unwrap()
                * point![<Ground>, 2.0, 0.0],
            point![2.0, 0.0],
            epsilon = 1e-5
        );
    }

    #[test]
    fn localization_ahead_of_odometry_waits_for_its_pose() {
        let mut history = OdometryHistory::new(Duration::from_secs(1));
        history.push_pose(time(0), Pose2::zero());
        history.push_ground_to_field(time(0), Isometry2::from_parts(vector![1.0, 0.0], 0.0));
        history.push_ground_to_field(time(50), Isometry2::from_parts(vector![5.0, 0.0], 0.0));

        let first = history.odometry_to_field().unwrap();
        assert_relative_eq!(first.as_pose().position(), point![1.0, 0.0]);

        history.push_pose(time(60), Pose2::zero());
        let second = history.odometry_to_field().unwrap();
        assert_relative_eq!(second.as_pose().position(), point![5.0, 0.0]);
    }
}
