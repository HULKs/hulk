use bevy::{
    asset::RenderAssetUsages,
    camera::visibility::RenderLayers,
    prelude::*,
    render::render_resource::{Extent3d, TextureDimension, TextureFormat},
};
use types::{
    field_dimensions::{FieldDimensions, Half, Side},
    field_marks::{FieldMark, field_marks_from_field_dimensions},
};

const TEXTURE_SIZE: u32 = 128;

/// Spawns the field geometry and lighting shared by interactive and sensor views.
pub fn setup_field_scene(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut images: ResMut<Assets<Image>>,
) {
    let dimensions = FieldDimensions::SPL_2025;
    let texture = images.add(Image::new(
        Extent3d {
            width: TEXTURE_SIZE,
            height: TEXTURE_SIZE,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        field_texture_rgba(),
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::default(),
    ));
    let green = materials.add(StandardMaterial {
        base_color_texture: Some(texture),
        perceptual_roughness: 0.9,
        ..default()
    });
    let white = material(&mut materials, Color::WHITE, true);

    commands.spawn((
        PointLight {
            intensity: 2_000.0,
            range: 18.0,
            ..default()
        },
        Transform::from_xyz(0.0, 7.0, 0.0),
        RenderLayers::from_layers(&[0, 1]),
    ));

    commands.spawn((
        Mesh3d(meshes.add(Cuboid::new(
            dimensions.length + 2.0 * dimensions.border_strip_width,
            0.025,
            dimensions.width + 2.0 * dimensions.border_strip_width,
        ))),
        MeshMaterial3d(green),
        Transform::from_xyz(0.0, -0.015, 0.0),
    ));

    let mut segments = Vec::new();
    for mark in field_marks_from_field_dimensions(&dimensions) {
        match mark {
            FieldMark::Line { line, .. } => {
                segments.push(([line.0.x(), line.0.y()], [line.1.x(), line.1.y()]))
            }
            FieldMark::Circle { center, radius } => {
                for index in 0..48 {
                    let start = std::f32::consts::TAU * index as f32 / 48.0;
                    let end = std::f32::consts::TAU * (index + 1) as f32 / 48.0;
                    segments.push((
                        [
                            center.x() + radius * start.cos(),
                            center.y() + radius * start.sin(),
                        ],
                        [
                            center.x() + radius * end.cos(),
                            center.y() + radius * end.sin(),
                        ],
                    ));
                }
            }
        }
    }
    for (start, end) in segments {
        spawn_line(
            &mut commands,
            &mut meshes,
            white.clone(),
            start,
            end,
            dimensions.line_width,
        );
    }

    for half in [Half::Own, Half::Opponent] {
        for side in [Side::Right, Side::Left] {
            let post = dimensions.goal_post(half, side);
            commands.spawn((
                Mesh3d(meshes.add(Cylinder::new(dimensions.goal_post_diameter / 2.0, 0.8))),
                MeshMaterial3d(white.clone()),
                Transform::from_xyz(post.x(), 0.4, -post.y()),
            ));
        }
    }
}

fn material(
    materials: &mut Assets<StandardMaterial>,
    color: Color,
    unlit: bool,
) -> Handle<StandardMaterial> {
    materials.add(StandardMaterial {
        base_color: color,
        unlit,
        perceptual_roughness: 0.9,
        ..default()
    })
}

fn spawn_line(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    material: Handle<StandardMaterial>,
    start: [f32; 2],
    end: [f32; 2],
    width: f32,
) {
    let dx = end[0] - start[0];
    let dy = end[1] - start[1];
    commands.spawn((
        Mesh3d(meshes.add(Cuboid::new(dx.hypot(dy), 0.012, width))),
        MeshMaterial3d(material),
        Transform::from_xyz((start[0] + end[0]) / 2.0, 0.008, -(start[1] + end[1]) / 2.0)
            .with_rotation(Quat::from_rotation_y(dy.atan2(dx))),
    ));
}

fn field_texture_rgba() -> Vec<u8> {
    let mut pixels = Vec::with_capacity((TEXTURE_SIZE * TEXTURE_SIZE * 4) as usize);
    for y in 0..TEXTURE_SIZE {
        for x in 0..TEXTURE_SIZE {
            let hash = x
                .wrapping_mul(0x9e37_79b9)
                .wrapping_add(y.wrapping_mul(0x85eb_ca6b));
            let grain = ((hash ^ (hash >> 16)) & 31) as u8;
            let stripe = if (x / 16) % 2 == 0 { 10 } else { 0 };
            pixels.extend_from_slice(&[8 + grain / 4, 70 + grain + stripe, 24 + grain / 2, 255]);
        }
    }
    pixels
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn field_texture_is_deterministic_and_textured() {
        let first = field_texture_rgba();
        let second = field_texture_rgba();

        assert_eq!(first, second);
        assert_eq!(first.len(), (TEXTURE_SIZE * TEXTURE_SIZE * 4) as usize);
        assert!(
            first
                .as_chunks::<4>()
                .0
                .iter()
                .any(|pixel| pixel != &first[..4])
        );
        assert!(first.as_chunks::<4>().0.iter().all(|pixel| pixel[3] == 255));
    }
}
