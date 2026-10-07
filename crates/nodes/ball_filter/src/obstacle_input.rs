//! Timestamped Ground obstacle selection and robot-motion compensation.
use std::{collections::BTreeMap, time::Duration};

use coordinate_systems::Odometry;
use linear_algebra::Pose2;
use ros_z::time::Time;
use types::{obstacles::Obstacle, odometry, time_wrapper::TimeWrapper};

const HISTORY_CAPACITY: usize = 512;
// Robot odometry normally arrives every 2 ms. Never infer a historical frame
// from an arbitrarily distant pose or from visual field localization.
const MAXIMUM_ODOMETRY_AGE: Duration = Duration::from_millis(2);

#[derive(Default)]
pub struct ObstacleHistory {
    samples: BTreeMap<Time, Vec<Obstacle>>,
}

impl ObstacleHistory {
    pub fn insert(&mut self, time: Time, obstacles: Vec<Obstacle>) {
        self.samples.insert(time, obstacles);
        while self.samples.len() > HISTORY_CAPACITY {
            self.samples.pop_first();
        }
    }

    pub fn at(&self, time: Time, maximum_age: Duration) -> Option<TimeWrapper<Vec<Obstacle>>> {
        let (&source_time, obstacles) = self.samples.range(..=time).next_back()?;
        (time.duration_since(source_time) <= maximum_age).then(|| TimeWrapper {
            time: source_time,
            inner: obstacles.clone(),
        })
    }
}

#[derive(Default)]
pub struct OdometryHistory {
    poses: BTreeMap<Time, Pose2<Odometry>>,
}

impl OdometryHistory {
    pub fn insert(&mut self, time: Time, pose: Pose2<Odometry>) {
        self.poses.insert(time, pose);
        while self.poses.len() > HISTORY_CAPACITY {
            self.poses.pop_first();
        }
    }

    fn at(&self, time: Time) -> Option<Pose2<Odometry>> {
        let (&pose_time, &pose) = self.poses.range(..=time).next_back()?;
        (time.duration_since(pose_time) <= MAXIMUM_ODOMETRY_AGE).then_some(pose)
    }

    pub fn align(
        &self,
        time: Time,
        obstacles: Option<&TimeWrapper<Vec<Obstacle>>>,
        maximum_age: Duration,
    ) -> Option<Vec<Obstacle>> {
        let obstacles = obstacles?;
        if obstacles.time > time || time.duration_since(obstacles.time) > maximum_age {
            return None;
        }
        if obstacles.time == time {
            return Some(obstacles.inner.clone());
        }
        let source = self.at(obstacles.time)?;
        let current = self.at(time)?;
        let source_to_current = odometry::previous_to_current(source, current);
        Some(
            obstacles
                .inner
                .iter()
                .map(|obstacle| Obstacle {
                    position: source_to_current * obstacle.position,
                    ..*obstacle
                })
                .collect(),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use linear_algebra::point;

    fn time(millis: i64) -> Time {
        Time::from_nanos(millis * 1_000_000)
    }

    #[test]
    fn selection_is_causal_bounded_and_preserves_exact_replaced_snapshot() {
        let mut history = ObstacleHistory::default();
        history.insert(time(40), vec![Obstacle::robot(point![1.0, 0.0], 0.2, 0.3)]);
        history.insert(time(80), vec![Obstacle::robot(point![2.0, 0.0], 0.2, 0.3)]);
        assert!(history.at(time(39), Duration::from_millis(100)).is_none());
        let selected = history.at(time(60), Duration::from_millis(100)).unwrap();
        assert_eq!(selected.time, time(40));
        assert_eq!(selected.inner[0].position, point![1.0, 0.0]);
        history.insert(time(40), vec![]);
        assert!(
            history
                .at(time(60), Duration::from_millis(100))
                .unwrap()
                .inner
                .is_empty()
        );
        assert_eq!(
            selected.inner.len(),
            1,
            "selected diagnostic owns the original snapshot"
        );
        assert!(history.at(time(181), Duration::from_millis(100)).is_none());
    }

    #[test]
    fn obstacles_follow_full_robot_translation_and_rotation_between_frames() {
        let mut history = OdometryHistory::default();
        history.insert(time(0), Pose2::new(point![0.0, 0.0], 0.0));
        history.insert(
            time(40),
            Pose2::new(point![1.0, 0.0], std::f32::consts::FRAC_PI_2),
        );
        let original = TimeWrapper {
            time: time(0),
            inner: vec![Obstacle::robot(point![2.0, 0.0], 0.2, 0.3)],
        };
        let aligned = history
            .align(time(40), Some(&original), Duration::from_millis(100))
            .unwrap();
        assert!((aligned[0].position - point![0.0, -1.0]).norm() < 1e-6);
        assert_eq!(original.inner[0].position, point![2.0, 0.0]);
        assert!(
            history
                .align(time(43), Some(&original), Duration::from_millis(100))
                .is_none()
        );
        assert!(
            history
                .align(time(40), Some(&original), Duration::from_millis(39))
                .is_none()
        );
        assert!(
            history
                .align(time(40), None, Duration::from_millis(100))
                .is_none()
        );
        let future = TimeWrapper {
            time: time(41),
            inner: vec![],
        };
        assert!(
            history
                .align(time(40), Some(&future), Duration::from_millis(100))
                .is_none()
        );
    }
}
