//! Relates Ground frames through timestamped odometry and localization.

pub mod ground_alignment;
pub mod odometry_history;

#[cfg(test)]
mod tests {
    use super::{ground_alignment::GroundAlignment, odometry_history::OdometryHistory};
    use coordinate_systems::{Ground, Odometry};
    use linear_algebra::{Isometry2, Pose2, point, vector};
    use ros_z::time::Time;
    use std::time::Duration;

    fn time(ms: u64) -> Time {
        Time::zero() + Duration::from_millis(ms)
    }

    #[test]
    fn delayed_localization_and_held_data_share_their_source_frame() {
        let mut frames = GroundAlignment::default();
        for ms in (0..=100).step_by(2) {
            frames.push_pose(
                time(ms),
                Pose2::<Odometry>::new(point![ms as f32 * 0.01, 0.0], ms as f32 * 0.01),
            );
        }
        frames.push_ground_to_field(time(0), Isometry2::from_parts(vector![1.0, 2.0], 0.0));
        let position = point![<Ground>, 3.0, 0.0];
        let expected = point![4.0, 2.0];
        let current = frames.current_ground_to_field().unwrap();
        let moved = frames.in_current_ground(time(0), position).unwrap();
        assert!((current * moved - expected).norm() < 1e-5);
        assert!((current * position - expected).norm() > 1.0);
        frames.expire_if_silent(time(0));
        frames.expire_if_silent(time(501));
        assert!(frames.current_frame().is_none());
    }

    #[test]
    fn pose_lookup_bounds_interpolation_gaps_and_retention() {
        let mut poses = OdometryHistory::new(Duration::from_secs(2));
        let tolerance = Duration::from_millis(10);
        poses.push_pose(time(0), Pose2::zero());
        poses.push_pose(time(2), Pose2::new(point![0.02, 0.0], 0.02));
        assert!((poses.pose_near(time(1), tolerance).unwrap().position().x() - 0.01).abs() < 1e-5);
        assert!(poses.pose_near(time(13), tolerance).is_none());
        poses.push_pose(time(1_000), Pose2::zero());
        assert!(poses.pose_near(time(500), tolerance).is_none());
        poses.push_ground_to_field(time(500), Isometry2::identity());
        assert!(poses.current_ground_to_field().is_none());
        poses.push_pose(time(3_000), Pose2::zero());
        assert!(poses.pose_near(time(0), tolerance).is_none());
    }
}
