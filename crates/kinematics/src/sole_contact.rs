use coordinate_systems::Robot;
use linear_algebra::{Isometry3, Orientation3, Point3, Vector2, nalgebra};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SoleSide {
    Left,
    Right,
}

#[derive(Clone, Copy, Debug)]
pub struct SoleContact {
    pub min_z: f32,
    pub contact_xy_in_leveled_robot: Vector2<Robot>,
    pub candidate_count: usize,
}

#[derive(Clone, Copy, Debug)]
pub struct SupportSelectionParameters {
    pub double_support_deadband: f32,
    pub support_switch_hysteresis: f32,
}

pub fn estimate_sole_contact<Sole>(
    sole_to_robot: Isometry3<Sole, Robot>,
    roll: f32,
    pitch: f32,
    vertices: &[Point3<Sole>],
    contact_height_epsilon: f32,
) -> Option<SoleContact> {
    if vertices.is_empty() {
        return None;
    }

    struct Level;

    let level_to_robot =
        Isometry3::<Level, Robot>::from(Orientation3::from_euler_angles(roll, pitch, 0.0).mirror());
    let robot_to_level = level_to_robot.inverse();

    let vertices_in_level = vertices
        .iter()
        .map(|vertex| robot_to_level * (sole_to_robot * *vertex))
        .collect::<Vec<_>>();
    let min_z = vertices_in_level
        .iter()
        .map(|vertex| vertex.z())
        .fold(f32::INFINITY, f32::min);

    let candidates = vertices_in_level
        .iter()
        .filter(|vertex| vertex.z() <= min_z + contact_height_epsilon)
        .collect::<Vec<_>>();
    let candidate_count = candidates.len();
    if candidate_count == 0 {
        return None;
    }

    let contact_xy_sum = candidates
        .iter()
        .map(|vertex| vertex.xy().coords().inner)
        .sum::<nalgebra::Vector2<f32>>();
    let contact_xy = contact_xy_sum / candidate_count as f32;

    Some(SoleContact {
        min_z,
        contact_xy_in_leveled_robot: Vector2::wrap(contact_xy),
        candidate_count,
    })
}

pub fn select_support_side(
    left_contact: SoleContact,
    right_contact: SoleContact,
    previous_support_side: Option<SoleSide>,
    parameters: SupportSelectionParameters,
) -> Option<SoleSide> {
    let height_difference = left_contact.min_z - right_contact.min_z;

    if height_difference.abs() < parameters.double_support_deadband {
        return None;
    }

    match previous_support_side {
        Some(SoleSide::Left) if height_difference < parameters.support_switch_hysteresis => {
            Some(SoleSide::Left)
        }
        Some(SoleSide::Right) if height_difference > -parameters.support_switch_hysteresis => {
            Some(SoleSide::Right)
        }
        _ if height_difference < 0.0 => Some(SoleSide::Left),
        _ => Some(SoleSide::Right),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use linear_algebra::{IntoTransform, nalgebra, point, vector};

    #[derive(Clone, Copy, Debug)]
    struct Sole;

    fn vertices() -> [Point3<Sole>; 4] {
        [
            point![<Sole>, 0.1, 0.05, 0.0],
            point![<Sole>, 0.1, -0.05, 0.0],
            point![<Sole>, -0.1, 0.05, 0.0],
            point![<Sole>, -0.1, -0.05, 0.0],
        ]
    }

    #[test]
    fn flat_sole_uses_all_vertices_and_center_contact() {
        let contact = estimate_sole_contact(Isometry3::identity(), 0.0, 0.0, &vertices(), 0.001)
            .expect("flat sole has a contact");

        assert_eq!(contact.candidate_count, 4);
        assert!((contact.contact_xy_in_leveled_robot.x() - 0.0).abs() < 1.0e-6);
        assert!((contact.contact_xy_in_leveled_robot.y() - 0.0).abs() < 1.0e-6);
    }

    #[test]
    fn toe_down_pitch_uses_front_vertices() {
        let sole_to_robot = nalgebra::Isometry3::from_parts(
            nalgebra::Translation3::identity(),
            nalgebra::UnitQuaternion::from_euler_angles(0.0, 0.2, 0.0),
        )
        .framed_transform();

        let contact = estimate_sole_contact(sole_to_robot, 0.0, 0.0, &vertices(), 0.001)
            .expect("toe-down sole has a contact");

        assert_eq!(contact.candidate_count, 2);
        assert!(contact.contact_xy_in_leveled_robot.x() > 0.09);
    }

    #[test]
    fn heel_down_pitch_uses_rear_vertices() {
        let sole_to_robot = nalgebra::Isometry3::from_parts(
            nalgebra::Translation3::identity(),
            nalgebra::UnitQuaternion::from_euler_angles(0.0, -0.2, 0.0),
        )
        .framed_transform();

        let contact = estimate_sole_contact(sole_to_robot, 0.0, 0.0, &vertices(), 0.001)
            .expect("heel-down sole has a contact");

        assert_eq!(contact.candidate_count, 2);
        assert!(contact.contact_xy_in_leveled_robot.x() < -0.09);
    }

    #[test]
    fn side_down_roll_uses_left_vertices() {
        let sole_to_robot = nalgebra::Isometry3::from_parts(
            nalgebra::Translation3::identity(),
            nalgebra::UnitQuaternion::from_euler_angles(0.2, 0.0, 0.0),
        )
        .framed_transform();

        let contact = estimate_sole_contact(sole_to_robot, 0.0, 0.0, &vertices(), 0.001)
            .expect("side-down sole has a contact");

        assert_eq!(contact.candidate_count, 2);
        assert!(contact.contact_xy_in_leveled_robot.y().abs() > 0.04);
    }

    #[test]
    fn corner_down_combined_roll_and_pitch_uses_one_vertex() {
        let sole_to_robot = nalgebra::Isometry3::from_parts(
            nalgebra::Translation3::identity(),
            nalgebra::UnitQuaternion::from_euler_angles(0.2, 0.2, 0.0),
        )
        .framed_transform();

        let contact = estimate_sole_contact(sole_to_robot, 0.0, 0.0, &vertices(), 0.001)
            .expect("corner-down sole has a contact");

        assert_eq!(contact.candidate_count, 1);
        assert!(contact.contact_xy_in_leveled_robot.x() > 0.09);
        assert!(contact.contact_xy_in_leveled_robot.y().abs() > 0.04);
    }

    #[test]
    fn double_support_deadband_returns_none() {
        let left = SoleContact {
            min_z: 0.0,
            contact_xy_in_leveled_robot: vector![<Robot>, 0.0, 0.0],
            candidate_count: 4,
        };
        let right = SoleContact {
            min_z: 0.002,
            contact_xy_in_leveled_robot: vector![<Robot>, 0.0, 0.0],
            candidate_count: 4,
        };

        let side = select_support_side(
            left,
            right,
            None,
            SupportSelectionParameters {
                double_support_deadband: 0.003,
                support_switch_hysteresis: 0.004,
            },
        );

        assert_eq!(side, None);
    }

    #[test]
    fn hysteresis_keeps_previous_side_near_switch_threshold() {
        let left = SoleContact {
            min_z: 0.003,
            contact_xy_in_leveled_robot: vector![<Robot>, 0.0, 0.0],
            candidate_count: 2,
        };
        let right = SoleContact {
            min_z: 0.0,
            contact_xy_in_leveled_robot: vector![<Robot>, 0.0, 0.0],
            candidate_count: 2,
        };

        let side = select_support_side(
            left,
            right,
            Some(SoleSide::Left),
            SupportSelectionParameters {
                double_support_deadband: 0.001,
                support_switch_hysteresis: 0.004,
            },
        );

        assert_eq!(side, Some(SoleSide::Left));
    }
}
