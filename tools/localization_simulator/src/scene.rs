use bevy::{
    asset::RenderAssetUsages,
    camera::visibility::RenderLayers,
    camera_controller::pan_orbit_camera::prelude::PanOrbitCamera,
    mesh::{PrimitiveTopology, VertexAttributeValues},
    prelude::*,
};
use localization_simulator::bevy_scene::setup_field_scene;
use nalgebra::{Isometry3, Matrix3, Rotation3, UnitQuaternion, Vector3};

const TRUTH: Color = Color::srgb(0.0, 0.9, 0.95);
const BACKEND: Color = Color::srgb(1.0, 0.82, 0.05);
const LIVE: Color = Color::srgb(1.0, 0.05, 0.75);

#[derive(Clone, Default, Resource)]
pub(crate) struct SceneState {
    pub revision: u64,
    pub truth: Option<Isometry3<f32>>,
    pub backend: Option<Isometry3<f32>>,
    pub live: Option<Isometry3<f32>>,
    pub truth_history: Vec<Isometry3<f32>>,
    pub backend_history: Vec<Isometry3<f32>>,
    pub live_history: Vec<Isometry3<f32>>,
}

#[derive(Clone, Copy, Component)]
enum PoseKind {
    Truth,
    Backend,
    Live,
}

#[derive(Component)]
struct Trail(PoseKind);

pub(crate) fn configure(app: &mut App) {
    app.insert_resource(SceneState::default())
        .insert_resource(GlobalAmbientLight {
            color: Color::WHITE,
            brightness: 450.0,
            ..default()
        })
        .add_systems(Startup, (setup_field_scene, setup_overlays))
        .add_systems(
            Update,
            (position_camera_once, update_markers, update_trails),
        );
}

fn setup_overlays(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    for (kind, color) in [
        (PoseKind::Truth, TRUTH),
        (PoseKind::Backend, BACKEND),
        (PoseKind::Live, LIVE),
    ] {
        let pose = commands
            .spawn((kind, Transform::default(), Visibility::Hidden))
            .id();
        commands.entity(pose).with_children(|parent| {
            parent.spawn((
                Mesh3d(meshes.add(Cuboid::new(0.28, 0.16, 0.2))),
                MeshMaterial3d(material(&mut materials, color, false)),
                Transform::default(),
                RenderLayers::layer(1),
            ));
            parent.spawn((
                Mesh3d(meshes.add(Cuboid::new(0.26, 0.07, 0.07))),
                MeshMaterial3d(material(&mut materials, color, true)),
                Transform::from_xyz(0.25, 0.0, 0.0),
                RenderLayers::layer(1),
            ));
        });
        commands.spawn((
            Trail(kind),
            Mesh3d(meshes.add(empty_line_mesh())),
            MeshMaterial3d(material(&mut materials, color, true)),
            RenderLayers::layer(1),
        ));
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

fn position_camera_once(
    mut commands: Commands,
    mut positioned: Local<bool>,
    mut cameras: Query<(Entity, &mut Transform, &mut PanOrbitCamera), With<Camera3d>>,
) {
    if *positioned {
        return;
    }
    for (entity, mut transform, mut camera) in &mut cameras {
        *transform = Transform::from_xyz(6.5, 7.5, 8.5).looking_at(Vec3::ZERO, Vec3::Y);
        camera.last_anchor_depth = -(transform.translation.length() as f64);
        commands
            .entity(entity)
            .insert(RenderLayers::from_layers(&[0, 1]));
        *positioned = true;
    }
}

fn update_markers(
    state: Res<SceneState>,
    mut markers: Query<(&PoseKind, &mut Transform, &mut Visibility)>,
) {
    if !state.is_changed() {
        return;
    }
    for (kind, mut transform, mut visibility) in &mut markers {
        let pose = match kind {
            PoseKind::Truth => state.truth.as_ref(),
            PoseKind::Backend => state.backend.as_ref(),
            PoseKind::Live => state.live.as_ref(),
        };
        if let Some(pose) = pose {
            *transform = bevy_transform(pose);
            *visibility = Visibility::Visible;
        } else {
            *visibility = Visibility::Hidden;
        }
    }
}

fn update_trails(
    state: Res<SceneState>,
    trails: Query<(&Trail, &Mesh3d)>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut previous: Local<(u64, [usize; 3])>,
) {
    if !state.is_changed() {
        return;
    }
    for (trail, mesh_handle) in &trails {
        let poses = match trail.0 {
            PoseKind::Truth => &state.truth_history,
            PoseKind::Backend => &state.backend_history,
            PoseKind::Live => &state.live_history,
        };
        if let Some(mut mesh) = meshes.get_mut(&mesh_handle.0) {
            let index = match trail.0 {
                PoseKind::Truth => 0,
                PoseKind::Backend => 1,
                PoseKind::Live => 2,
            };
            let rebuild = previous.0 != state.revision
                || poses.len() < previous.1[index]
                || mesh.attribute(Mesh::ATTRIBUTE_POSITION).is_none();
            if rebuild {
                let mut positions = Vec::with_capacity(poses.len().saturating_sub(1) * 2);
                for pair in poses.windows(2) {
                    positions.push(bevy_position(pair[0].translation.vector));
                    positions.push(bevy_position(pair[1].translation.vector));
                }
                mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
            } else if poses.len() > previous.1[index] {
                let Some(VertexAttributeValues::Float32x3(positions)) =
                    mesh.attribute_mut(Mesh::ATTRIBUTE_POSITION)
                else {
                    continue;
                };
                for current in previous.1[index].max(1)..poses.len() {
                    positions.push(bevy_position(poses[current - 1].translation.vector));
                    positions.push(bevy_position(poses[current].translation.vector));
                }
            }
            previous.1[index] = poses.len();
        }
    }
    previous.0 = state.revision;
}

fn empty_line_mesh() -> Mesh {
    Mesh::new(
        PrimitiveTopology::LineList,
        RenderAssetUsages::MAIN_WORLD | RenderAssetUsages::RENDER_WORLD,
    )
}

fn bevy_transform(pose: &Isometry3<f32>) -> Transform {
    let basis = Matrix3::new(1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, -1.0, 0.0);
    let rotation = basis * pose.rotation.to_rotation_matrix().matrix() * basis.transpose();
    let quaternion =
        UnitQuaternion::from_rotation_matrix(&Rotation3::from_matrix_unchecked(rotation));
    let quaternion = quaternion.quaternion();
    Transform {
        translation: Vec3::from(bevy_position(pose.translation.vector)),
        rotation: Quat::from_xyzw(quaternion.i, quaternion.j, quaternion.k, quaternion.w),
        ..default()
    }
}

fn bevy_position(position: Vector3<f32>) -> [f32; 3] {
    [position.x, position.z, -position.y]
}
