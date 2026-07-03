use coordinate_systems::{Field, Robot};
use factrs::{core::SO3, variables::SE23};
use linear_algebra::Isometry3;
use nalgebra::Vector3;

pub fn robot_to_field_to_se23(robot_to_field: Isometry3<Robot, Field>) -> SE23<f64> {
    let rotation = robot_to_field.inner.rotation.quaternion();

    SE23::from_rot_vel_trans(
        SO3::from_xyzw(
            rotation.i as f64,
            rotation.j as f64,
            rotation.k as f64,
            rotation.w as f64,
        ),
        Vector3::zeros(),
        robot_to_field.inner.translation.vector.cast(),
    )
}

#[cfg(test)]
mod tests {
    use linear_algebra::Orientation3;

    use super::*;

    #[test]
    fn robot_to_field_conversion_preserves_pose_with_zero_velocity() {
        let rotation: Orientation3<Field> = Orientation3::from_euler_angles(0.1, -0.2, 0.3);
        let robot_to_field: Isometry3<Robot, Field> = linear_algebra::Isometry3::from_parts(
            linear_algebra::vector![<Field>, 1.0, 2.0, 0.4],
            rotation,
        );

        let pose = robot_to_field_to_se23(robot_to_field);

        assert!((pose.xyz() - nalgebra::vector![1.0, 2.0, 0.4]).norm() < 1.0e-6);
        assert!((pose.uvw() - nalgebra::vector![0.0, 0.0, 0.0]).norm() < 1.0e-9);
        assert!((pose.rot().w() - rotation.inner.w as f64).abs() < 1.0e-9);
        assert!((pose.rot().x() - rotation.inner.i as f64).abs() < 1.0e-9);
        assert!((pose.rot().y() - rotation.inner.j as f64).abs() < 1.0e-9);
        assert!((pose.rot().z() - rotation.inner.k as f64).abs() < 1.0e-9);
    }
}
