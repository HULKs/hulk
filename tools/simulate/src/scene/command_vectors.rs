//! Sent motion commands in world coordinates, independent of camera orientation.
use bevy::{light::NotShadowCaster, prelude::*, transform::TransformSystems};
use types::motion_command::MotionCommand;

use super::ball::{self, SpawnedBalls};
use crate::{
    bevy_mujoco::MujocoWorld, robot_io::RobotBinding, robotics::Robotics,
    simulation::ControlledRobot,
};

pub struct CommandVectorsPlugin;

impl Plugin for CommandVectorsPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, setup)
            .add_systems(PostUpdate, update.before(TransformSystems::Propagate));
    }
}

#[derive(Clone, Copy, Component)]
enum VectorKind {
    Linear,
    Angular,
    Kick,
}

#[derive(Component)]
struct ArrowPart {
    kind: VectorKind,
    tip: bool,
}

#[derive(Clone, Copy, Debug)]
struct Arrow {
    origin: Vec3,
    vector: Vec3,
}

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let shaft = meshes.add(Cylinder::new(1.0, 1.0));
    let tip = meshes.add(Cone {
        radius: 1.0,
        height: 1.0,
    });
    for (kind, color) in [
        (VectorKind::Linear, Color::srgb(0.2, 0.55, 1.0)),
        (VectorKind::Angular, Color::srgb(0.2, 0.9, 0.4)),
        (VectorKind::Kick, Color::srgb(1.0, 0.65, 0.15)),
    ] {
        let material = materials.add(StandardMaterial {
            base_color: color,
            unlit: true,
            ..default()
        });
        for is_tip in [false, true] {
            commands.spawn((
                ArrowPart { kind, tip: is_tip },
                Mesh3d(if is_tip { tip.clone() } else { shaft.clone() }),
                MeshMaterial3d(material.clone()),
                Transform::default(),
                Visibility::Hidden,
                NotShadowCaster,
                Pickable::IGNORE,
            ));
        }
    }
}

fn scene_point(point: nalgebra::Point3<f32>) -> Vec3 {
    Vec3::new(point.x, point.z, -point.y)
}

fn scene_vector(vector: nalgebra::Vector3<f32>) -> Vec3 {
    Vec3::new(vector.x, vector.z, -vector.y)
}

fn vectors(
    command: &MotionCommand,
    ground_to_world: nalgebra::Isometry3<f32>,
    robot_position: nalgebra::Point3<f32>,
    ball_height: f32,
) -> [Option<Arrow>; 3] {
    let mut result = [None; 3];
    match command {
        MotionCommand::WalkWithVelocity {
            velocity,
            angular_velocity,
            ..
        } => {
            // Ground axes use robot yaw, not its roll/pitch. Length is 1 m per unit
            // of commanded speed; the elevated origin keeps arrows clear of feet.
            let origin = scene_point(robot_position) + Vec3::Y * 0.2;
            result[0] = Some(Arrow {
                origin,
                vector: scene_vector(
                    ground_to_world.rotation * nalgebra::vector![velocity.x(), velocity.y(), 0.0],
                ),
            });
            result[1] = Some(Arrow {
                origin,
                vector: Vec3::Y * *angular_velocity,
            });
        }
        MotionCommand::VisualKick {
            ball_position,
            kick_direction,
            ..
        } => {
            let angle = kick_direction.angle();
            result[2] = Some(Arrow {
                origin: scene_point(
                    ground_to_world
                        * nalgebra::point![ball_position.x(), ball_position.y(), ball_height],
                ),
                // The command carries a direction, not a numeric kick speed.
                vector: scene_vector(
                    ground_to_world.rotation * nalgebra::vector![angle.cos(), angle.sin(), 0.0],
                ),
            });
        }
        _ => {}
    }
    result
}

fn part_transform(arrow: Arrow, tip: bool) -> Option<Transform> {
    let length = arrow.vector.length();
    if !arrow.origin.is_finite()
        || !arrow.vector.is_finite()
        || !length.is_finite()
        || length < 1e-4
    {
        return None;
    }
    let direction = arrow.vector / length;
    let tip_length = (length * 0.3).min(0.14);
    let radius = (length * 0.12).min(0.04);
    let shaft_length = length - tip_length;
    let (center, scale) = if tip {
        (
            length - tip_length * 0.5,
            Vec3::new(radius, tip_length, radius),
        )
    } else {
        (
            shaft_length * 0.5,
            Vec3::new(radius * 0.35, shaft_length, radius * 0.35),
        )
    };
    Some(Transform {
        translation: arrow.origin + direction * center,
        rotation: Quat::from_rotation_arc(Vec3::Y, direction),
        scale,
    })
}

fn update(
    world: Res<MujocoWorld>,
    io: Res<Robotics>,
    balls: Res<SpawnedBalls>,
    robot: Single<Entity, With<ControlledRobot>>,
    mut parts: Query<(&ArrowPart, &mut Transform, &mut Visibility)>,
) {
    let data = world.data();
    let prefix = format!("object_{}_", robot.to_bits());
    let arrows = RobotBinding::new(data, &prefix)
        .ok()
        .and_then(|binding| {
            let position = data.body(&format!("{prefix}Trunk"))?.view(data).xpos;
            let height = ball::first_position(&world, &balls)
                .ok()
                .map(|position| position[2] as f32);
            if matches!(io.input_motion, MotionCommand::VisualKick { .. }) && height.is_none() {
                return None;
            }
            Some(vectors(
                &io.input_motion,
                binding.ground_to_world(data),
                nalgebra::point![position[0] as f32, position[1] as f32, position[2] as f32],
                height.unwrap_or(0.0),
            ))
        })
        .unwrap_or([None; 3]);
    for (part, mut transform, mut visibility) in &mut parts {
        let index = match part.kind {
            VectorKind::Linear => 0,
            VectorKind::Angular => 1,
            VectorKind::Kick => 2,
        };
        if let Some(next) = arrows[index].and_then(|arrow| part_transform(arrow, part.tip)) {
            *transform = next;
            *visibility = Visibility::Visible;
        } else {
            *visibility = Visibility::Hidden;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use linear_algebra::{Orientation2, point, vector};
    use types::motion_command::{HeadMotion, KickPower};

    #[test]
    fn vectors_follow_ground_yaw_and_keep_their_respective_origins() {
        let ground = nalgebra::Isometry3::new(
            nalgebra::vector![2.0, 3.0, 0.0],
            nalgebra::vector![0.0, 0.0, std::f32::consts::FRAC_PI_2],
        );
        let robot = nalgebra::point![2.0, 3.0, 0.6];
        let walk = MotionCommand::WalkWithVelocity {
            head: HeadMotion::ZeroAngles,
            velocity: vector![0.4, -0.2],
            angular_velocity: -0.5,
        };
        let arrows = vectors(&walk, ground, robot, 0.105);
        let linear = arrows[0].unwrap();
        assert!(linear.origin.distance(Vec3::new(2.0, 0.8, -3.0)) < 1e-6);
        assert!(linear.vector.distance(Vec3::new(0.2, 0.0, -0.4)) < 1e-6);
        assert_eq!(arrows[1].unwrap().vector, Vec3::new(0.0, -0.5, 0.0));
        assert!(arrows[2].is_none());
        let kick = MotionCommand::VisualKick {
            head: HeadMotion::ZeroAngles,
            ball_position: point![1.0, 2.0],
            kick_direction: Orientation2::new(-std::f32::consts::FRAC_PI_2),
            target_position: point![0.0, 0.0],
            robot_theta_to_field: Orientation2::identity(),
            kick_power: KickPower::default(),
        };
        let arrow = vectors(&kick, ground, robot, 0.105)[2].unwrap();
        assert!(arrow.origin.distance(Vec3::new(0.0, 0.105, -4.0)) < 1e-6);
        assert!(arrow.vector.distance(Vec3::X) < 1e-6);
        let tip = part_transform(arrow, true).unwrap();
        assert!(
            (tip.translation + tip.rotation * Vec3::Y * tip.scale.y * 0.5)
                .distance(arrow.origin + arrow.vector)
                < 1e-6
        );
    }

    #[test]
    fn zero_and_invalid_vectors_are_hidden() {
        for vector in [
            Vec3::ZERO,
            Vec3::splat(f32::NAN),
            Vec3::splat(f32::INFINITY),
        ] {
            assert!(
                part_transform(
                    Arrow {
                        origin: Vec3::ZERO,
                        vector
                    },
                    true
                )
                .is_none()
            );
        }
        assert!(
            vectors(
                &MotionCommand::Damping,
                nalgebra::Isometry3::identity(),
                nalgebra::Point3::origin(),
                0.105
            )
            .iter()
            .all(Option::is_none)
        );
    }
}
