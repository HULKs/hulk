use std::f32::consts::{FRAC_PI_2, PI, TAU};

use bevy::{
    asset::RenderAssetUsages,
    mesh::{Indices, PrimitiveTopology},
    prelude::*,
};
use types::{
    field_dimensions::FieldDimensions,
    field_marks::{FieldMark, field_marks_from_field_dimensions},
};

use super::{goal, visual::ObjectVisualAssets};
use crate::{
    bevy_mujoco::MujocoWorld, parameters::CurrentSimulatorParameters,
    scene::SceneParameterUpdateSet,
};

#[derive(Debug, Component)]
pub struct Field;

#[derive(Component)]
struct FieldGoal(usize);

#[derive(Component)]
struct FieldMarkings;

#[derive(Component)]
pub struct FieldDropTarget;

pub struct FieldPlugin;

impl Plugin for FieldPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, spawn_field)
            .add_systems(PreUpdate, update_field.in_set(SceneParameterUpdateSet));
    }
}

fn spawn_field(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    object_assets: Res<ObjectVisualAssets>,
    parameters: Res<CurrentSimulatorParameters>,
) {
    let dimensions = parameters.parameters.field_dimensions;

    commands.spawn((
        FieldDropTarget,
        Pickable::default(),
        Field,
        Mesh3d(meshes.add(Plane3d::default().mesh().size(1.0, 1.0))),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: Color::srgb(0.04, 0.34, 0.13),
            perceptual_roughness: 0.95,
            ..default()
        })),
        Transform::from_scale(field_scale(&dimensions)),
    ));

    commands.spawn((
        FieldMarkings,
        Mesh3d(meshes.add(field_mesh(&dimensions))),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: Color::WHITE,
            unlit: true,
            cull_mode: None,
            depth_bias: 1.0,
            ..default()
        })),
    ));

    for (index, transform) in goal_transforms(&dimensions).into_iter().enumerate() {
        let entity = goal::spawn(&mut commands, &object_assets.goal, transform);
        commands.entity(entity).insert(FieldGoal(index));
    }
}

fn goal_transforms(dimensions: &FieldDimensions) -> [Transform; 2] {
    let half_length = dimensions.length / 2.0;
    [
        Transform::from_xyz(-half_length, 0.0, 0.0),
        Transform::from_xyz(half_length, 0.0, 0.0).with_rotation(Quat::from_rotation_y(PI)),
    ]
}

fn update_field(
    parameters: Res<CurrentSimulatorParameters>,
    mut field: Single<&mut Transform, (With<Field>, Without<FieldGoal>)>,
    markings: Single<&Mesh3d, With<FieldMarkings>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut goals: Query<(Entity, &FieldGoal, &mut Transform), Without<Field>>,
    mut mujoco: ResMut<MujocoWorld>,
) {
    if !parameters.is_changed() {
        return;
    }
    let dimensions = &parameters.parameters.field_dimensions;

    field.scale = field_scale(dimensions);
    meshes
        .insert(markings.id(), field_mesh(dimensions))
        .expect("field markings mesh should exist");

    let transforms = goal_transforms(dimensions);
    for (entity, goal, mut transform) in &mut goals {
        *transform = transforms[goal.0];
        if mujoco.contains_object(entity) {
            mujoco
                .set_object_pose(entity, *transform)
                .unwrap_or_else(|error| warn!("failed to move field goal: {error}"));
        }
    }
}

fn field_scale(dimensions: &FieldDimensions) -> Vec3 {
    Vec3::new(
        dimensions.length + 2.0 * dimensions.border_strip_width,
        1.0,
        dimensions.width + 2.0 * dimensions.border_strip_width,
    )
}

fn field_mesh(dimensions: &FieldDimensions) -> Mesh {
    let mut mesh = FieldMesh::default();

    for marking in field_marks_from_field_dimensions(dimensions) {
        match marking {
            FieldMark::Line { line, .. } => mesh.add_line(
                Vec2::new(line.0.x(), line.0.y()),
                Vec2::new(line.1.x(), line.1.y()),
                dimensions.line_width,
            ),
            FieldMark::Circle { center, radius } => mesh.add_arc(
                Vec2::new(center.x(), center.y()),
                radius,
                0.0,
                TAU,
                dimensions.line_width,
            ),
        }
    }

    let half_length = dimensions.length / 2.0;
    let half_width = dimensions.width / 2.0;
    for x_sign in [-1.0, 1.0] {
        for y_sign in [-1.0, 1.0] {
            mesh.add_arc(
                Vec2::new(x_sign * half_length, y_sign * half_width),
                dimensions.corner_arc_radius,
                if x_sign > 0.0 { PI } else { 0.0 },
                x_sign * y_sign * FRAC_PI_2,
                dimensions.line_width,
            );
        }
    }

    mesh.finish()
}

#[derive(Default)]
struct FieldMesh {
    positions: Vec<[f32; 3]>,
    indices: Vec<u32>,
}

impl FieldMesh {
    const HEIGHT: f32 = 0.002;

    fn add_line(&mut self, start: Vec2, end: Vec2, width: f32) {
        let offset = (end - start).perp().normalize_or_zero() * width / 2.0;
        self.add_quad([start - offset, end - offset, end + offset, start + offset]);
    }

    fn add_arc(&mut self, center: Vec2, radius: f32, start: f32, sweep: f32, width: f32) {
        if radius <= 0.0 {
            return;
        }

        let half_width = width / 2.0;
        let inner_radius = (radius - half_width).max(0.0);
        let outer_radius = radius + half_width;
        let segment_count = ((radius * sweep.abs() / 0.05).ceil() as usize).clamp(8, 96);

        for index in 0..segment_count {
            let angle = |index| start + sweep * index as f32 / segment_count as f32;
            self.add_quad([
                point_on_arc(center, inner_radius, angle(index)),
                point_on_arc(center, inner_radius, angle(index + 1)),
                point_on_arc(center, outer_radius, angle(index + 1)),
                point_on_arc(center, outer_radius, angle(index)),
            ]);
        }
    }

    fn add_quad(&mut self, points: [Vec2; 4]) {
        let base = self.positions.len() as u32;
        self.positions
            .extend(points.map(|point| [point.x, Self::HEIGHT, -point.y]));
        self.indices
            .extend([base, base + 1, base + 2, base, base + 2, base + 3]);
    }

    fn finish(self) -> Mesh {
        Mesh::new(
            PrimitiveTopology::TriangleList,
            RenderAssetUsages::RENDER_WORLD,
        )
        .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, self.positions)
        .with_inserted_indices(Indices::U32(self.indices))
    }
}

fn point_on_arc(center: Vec2, radius: f32, angle: f32) -> Vec2 {
    let (sin, cos) = angle.sin_cos();
    center + radius * Vec2::new(cos, sin)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn goals_are_placed_on_goal_lines_and_face_outward() {
        let [negative, positive] = goal_transforms(&FieldDimensions::SPL_2025);

        assert_eq!(negative.translation, Vec3::new(-4.5, 0.0, 0.0));
        assert_eq!(positive.translation, Vec3::new(4.5, 0.0, 0.0));
        assert!(
            (negative.rotation * Vec3::NEG_X).abs_diff_eq(Vec3::NEG_X, 1e-6),
            "negative-X goal net should extend toward negative X"
        );
        assert!(
            (positive.rotation * Vec3::NEG_X).abs_diff_eq(Vec3::X, 1e-6),
            "positive-X goal net should extend toward positive X"
        );
    }
}
