//! Brings ground data of different times into one picture.
//!
//! Ground moves with the robot, and messages arrive at their own rates and after their own delays,
//! so the latest samples of two topics generally do not refer to the same ground frame. Odometry
//! relates the ground frames of all times, and each localization estimate holds together with the
//! odometry pose of its own time. Together they place any ground frame in the field, as long as the
//! poses are known; without them, data is left out rather than drawn at some other place.

use std::time::Duration;

use coordinate_systems::{Field, Ground, Odometry};
use linear_algebra::{Isometry2, Pose2};
use ros_z::time::Time;
use ros_z_debug::{RetentionPolicy, TopicObservation};

use crate::backend::RobotBackend;

/// How long poses and localization estimates are kept for data arriving late.
const RETENTION: Duration = Duration::from_secs(2);
/// Poses are 2 ms apart; beyond this distance, the robot could have moved unnoticed.
const MAXIMUM_POSE_DISTANCE: Duration = Duration::from_millis(10);
/// How far a localization estimate may lie from the time of data without any odometry to bridge.
const MAXIMUM_LOCALIZATION_DISTANCE: Duration = Duration::from_millis(100);

pub struct GroundFrames {
    odometry: TopicObservation<Pose2<Odometry>>,
    ground_to_field: TopicObservation<Isometry2<Ground, Field>>,
}

impl GroundFrames {
    pub fn new(backend: &RobotBackend) -> Self {
        let _runtime_handle = backend.runtime_handle().enter();
        let retention =
            RetentionPolicy::time_window(RETENTION).expect("retention must be non-zero");
        Self {
            odometry: backend
                .observer()
                .observe_typed("inputs/odometry")
                .expect("failed to construct odometry observer")
                .retention(retention)
                .spawn(),
            ground_to_field: backend
                .observer()
                .observe_typed("ground_to_field")
                .expect("failed to construct ground_to_field observer")
                .retention(retention)
                .spawn(),
        }
    }

    /// The relation of all ground frames to the field and to the current one, as known right now.
    pub fn alignment(&self) -> GroundAlignment<'_> {
        GroundAlignment {
            frames: self,
            odometry_to_field: self.odometry_to_field(),
            current_pose: self.odometry.latest().map(|record| record.value),
        }
    }

    fn pose_at(&self, time: Time) -> Option<Pose2<Odometry>> {
        let record = self.odometry.get_nearest(time)?;
        (record.source_time.abs_diff(time) <= MAXIMUM_POSE_DISTANCE).then_some(record.value)
    }

    /// The newest localization estimate whose odometry pose is known, relative to the odometry.
    fn odometry_to_field(&self) -> Option<Isometry2<Odometry, Field>> {
        let anchor = |time, ground_to_field: Isometry2<Ground, Field>| {
            let pose = self.pose_at(time)?;
            Some(ground_to_field * pose.as_transform::<Ground>().inverse())
        };
        let latest = self.ground_to_field.latest()?;
        anchor(latest.source_time, latest.value).or_else(|| {
            self.ground_to_field
                .get_all()
                .iter()
                .rev()
                .find_map(|record| anchor(record.source_time, record.value))
        })
    }
}

pub struct GroundAlignment<'a> {
    frames: &'a GroundFrames,
    odometry_to_field: Option<Isometry2<Odometry, Field>>,
    current_pose: Option<Pose2<Odometry>>,
}

impl GroundAlignment<'_> {
    /// Where the robot's current ground frame is in the field.
    pub fn current_ground_to_field(&self) -> Option<Isometry2<Ground, Field>> {
        match self.current_pose {
            Some(pose) => Some(self.odometry_to_field? * pose.as_transform::<Ground>()),
            None => self
                .frames
                .ground_to_field
                .latest()
                .map(|record| record.value),
        }
    }

    /// Where the ground frame of `time` is in the field, by the latest localization.
    pub fn ground_to_field_at(&self, time: Time) -> Option<Isometry2<Ground, Field>> {
        if self.current_pose.is_none() {
            let record = self.frames.ground_to_field.get_nearest(time)?;
            return (record.source_time.abs_diff(time) <= MAXIMUM_LOCALIZATION_DISTANCE)
                .then_some(record.value);
        }
        Some(self.odometry_to_field? * self.frames.pose_at(time)?.as_transform::<Ground>())
    }

    /// Where the ground frame of `time` is in the current ground frame.
    ///
    /// Without any odometry, the current ground frame is unknown and the data is taken as current.
    pub fn ground_to_current_ground(&self, time: Time) -> Option<Isometry2<Ground, Ground>> {
        let Some(current_pose) = self.current_pose else {
            return Some(Isometry2::identity());
        };
        let pose = self.frames.pose_at(time)?;
        Some(current_pose.as_transform::<Ground>().inverse() * pose.as_transform::<Ground>())
    }
}
