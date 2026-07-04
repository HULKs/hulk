use coordinate_systems::{Ground, Odometry};
use linear_algebra::{Isometry2, Pose2};

pub fn current_to_previous(
    previous: Pose2<Odometry>,
    current: Pose2<Odometry>,
) -> Isometry2<Ground, Ground> {
    let previous_to_odometry = previous.as_transform::<Ground>();
    let current_to_odometry = current.as_transform::<Ground>();

    previous_to_odometry.inverse() * current_to_odometry
}

pub fn previous_to_current(
    previous: Pose2<Odometry>,
    current: Pose2<Odometry>,
) -> Isometry2<Ground, Ground> {
    current_to_previous(previous, current).inverse()
}

#[cfg(test)]
mod tests {
    use approx::assert_relative_eq;
    use coordinate_systems::Odometry;
    use linear_algebra::{Pose2, point};

    use super::{current_to_previous, previous_to_current};

    #[test]
    fn identical_poses_produce_identity_deltas() {
        let pose = Pose2::<Odometry>::new(point![<Odometry>, 1.0, 2.0], 0.3);

        let current_to_previous = current_to_previous(pose, pose);
        let previous_to_current = previous_to_current(pose, pose);

        assert_relative_eq!(current_to_previous.translation().x(), 0.0, epsilon = 1.0e-6);
        assert_relative_eq!(current_to_previous.translation().y(), 0.0, epsilon = 1.0e-6);
        assert_relative_eq!(
            current_to_previous.orientation().angle(),
            0.0,
            epsilon = 1.0e-6
        );
        assert_relative_eq!(previous_to_current.translation().x(), 0.0, epsilon = 1.0e-6);
        assert_relative_eq!(previous_to_current.translation().y(), 0.0, epsilon = 1.0e-6);
        assert_relative_eq!(
            previous_to_current.orientation().angle(),
            0.0,
            epsilon = 1.0e-6
        );
    }

    #[test]
    fn forward_motion_produces_opposite_direction_deltas() {
        let previous = Pose2::<Odometry>::new(point![<Odometry>, 0.0, 0.0], 0.0);
        let current = Pose2::<Odometry>::new(point![<Odometry>, 1.0, 0.0], 0.0);

        let current_to_previous = current_to_previous(previous, current);
        let previous_to_current = previous_to_current(previous, current);

        assert_relative_eq!(current_to_previous.translation().x(), 1.0, epsilon = 1.0e-6);
        assert_relative_eq!(current_to_previous.translation().y(), 0.0, epsilon = 1.0e-6);
        assert_relative_eq!(
            previous_to_current.translation().x(),
            -1.0,
            epsilon = 1.0e-6
        );
        assert_relative_eq!(previous_to_current.translation().y(), 0.0, epsilon = 1.0e-6);
    }

    #[test]
    fn rotation_wraparound_is_normalized_by_isometry_composition() {
        let previous =
            Pose2::<Odometry>::new(point![<Odometry>, 0.0, 0.0], std::f32::consts::PI - 0.1);
        let current =
            Pose2::<Odometry>::new(point![<Odometry>, 0.0, 0.0], -std::f32::consts::PI + 0.2);

        let current_to_previous = current_to_previous(previous, current);

        assert_relative_eq!(
            current_to_previous.orientation().angle(),
            0.3,
            epsilon = 1.0e-6
        );
    }
}
