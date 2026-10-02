use super::{
    CameraMatrix, Settings, ViewerData, convert_point, empty_mesh, robot_to_camera,
    robot_to_display, transform_from_isometry,
};
use bevy::{
    asset::RenderAssetUsages,
    mesh::{Indices, PrimitiveTopology},
    prelude::*,
};

use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
const CAMERA_VIEWPORT_DEPTH: f32 = 1.0;

type CameraViewportComponents = (
    &'static Mesh3d,
    &'static mut Transform,
    &'static mut Visibility,
);
type CameraImagePlaneFilter = (With<CameraImagePlane>, Without<CameraFrustum>);
#[derive(Component)]
pub(super) struct CameraFrustum;

#[derive(Component)]
pub(super) struct CameraImagePlane {
    texture: Handle<Image>,
    publication: Option<ros_z::pubsub::PublicationId>,
    valid: bool,
}

pub(super) fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut images: ResMut<Assets<Image>>,
) {
    let frustum_material = materials.add(StandardMaterial {
        base_color: Color::srgb(1.0, 0.85, 0.15),
        unlit: true,
        ..default()
    });
    let camera_image_texture = images.add(Image::transparent());
    let camera_image_material = materials.add(StandardMaterial {
        base_color: Color::srgba(1.0, 1.0, 1.0, 0.5),
        base_color_texture: Some(camera_image_texture.clone()),
        alpha_mode: AlphaMode::Blend,
        unlit: true,
        cull_mode: None,
        ..default()
    });
    commands.spawn((
        CameraFrustum,
        Mesh3d(meshes.add(empty_mesh(PrimitiveTopology::LineList))),
        MeshMaterial3d(frustum_material),
        Transform::default(),
        Visibility::Hidden,
    ));
    commands.spawn((
        CameraImagePlane {
            texture: camera_image_texture,
            publication: None,
            valid: false,
        },
        Mesh3d(meshes.add(empty_mesh(PrimitiveTopology::TriangleList))),
        MeshMaterial3d(camera_image_material),
        Transform::default(),
        Visibility::Hidden,
    ));
}
pub(super) fn update_camera_viewport(
    data: Res<ViewerData>,
    settings: Res<Settings>,
    mut frustum: Single<CameraViewportComponents, With<CameraFrustum>>,
    mut image_plane: Single<CameraViewportComponents, CameraImagePlaneFilter>,
    mut meshes: ResMut<Assets<Mesh>>,
    texture: Single<&CameraImagePlane>,
    mut geometry: Local<Option<([f32; 2], projection::intrinsic::Intrinsic)>>,
) {
    let Some(camera_matrix) = data.camera_matrix.as_ref().filter(|_| settings.camera) else {
        *frustum.2 = Visibility::Hidden;
        *image_plane.2 = Visibility::Hidden;
        return;
    };

    let transform = camera_to_display_transform(&data, camera_matrix);
    *frustum.1 = transform;
    *image_plane.1 = transform;
    *frustum.2 = Visibility::Visible;
    *image_plane.2 = if data.camera_frame.is_some() && texture.valid {
        Visibility::Visible
    } else {
        Visibility::Hidden
    };

    let key = (
        [camera_matrix.image_size.x(), camera_matrix.image_size.y()],
        camera_matrix.intrinsics,
    );
    if *geometry == Some(key) {
        return;
    }
    *geometry = Some(key);
    meshes
        .insert(frustum.0.id(), camera_frustum_mesh(camera_matrix))
        .expect("camera frustum mesh handle should be valid");
    meshes
        .insert(image_plane.0.id(), camera_image_plane_mesh(camera_matrix))
        .expect("camera image plane mesh handle should be valid");
}

pub(super) fn update_camera_image(
    data: Res<ViewerData>,
    settings: Res<Settings>,
    mut image_plane: Single<&mut CameraImagePlane>,
    mut images: ResMut<Assets<Image>>,
) {
    if !settings.camera {
        return;
    }
    let Some(frame) = &data.camera_frame else {
        return;
    };
    if image_plane.publication == Some(frame.publication_id) {
        return;
    }
    image_plane.publication = Some(frame.publication_id);
    image_plane.valid = false;
    if frame.value.width == 0 || frame.value.height == 0 {
        return;
    }
    let Ok(rgb) = image::RgbImage::try_from(frame.value.clone()) else {
        return;
    };
    let rgba = image::DynamicImage::ImageRgb8(rgb).into_rgba8();
    let image = Image::new(
        Extent3d {
            width: rgba.width(),
            height: rgba.height(),
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        rgba.into_raw(),
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::RENDER_WORLD,
    );
    images
        .insert(image_plane.texture.id(), image)
        .expect("camera texture exists");
    image_plane.valid = true;
}
fn camera_to_display_transform(data: &ViewerData, camera_matrix: &CameraMatrix) -> Transform {
    let robot_to_display = robot_to_display(data);
    let camera_to_robot = robot_to_camera(camera_matrix).inverse();

    transform_from_isometry(robot_to_display * camera_to_robot)
}
fn camera_frustum_mesh(camera_matrix: &CameraMatrix) -> Mesh {
    let corners = camera_viewport_corners(camera_matrix, CAMERA_VIEWPORT_DEPTH);
    let mut positions = Vec::with_capacity(16);

    for corner in corners {
        positions.push(camera_point(corner));
        positions.push(camera_point([0.0, 0.0, 0.0]));
    }
    for [start, end] in [[0, 1], [1, 2], [2, 3], [3, 0]] {
        positions.push(camera_point(corners[start]));
        positions.push(camera_point(corners[end]));
    }

    let mut mesh = Mesh::new(PrimitiveTopology::LineList, RenderAssetUsages::RENDER_WORLD);
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
    mesh
}

fn camera_image_plane_mesh(camera_matrix: &CameraMatrix) -> Mesh {
    let corners = camera_viewport_corners(camera_matrix, CAMERA_VIEWPORT_DEPTH);
    let mut mesh = Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::RENDER_WORLD,
    );
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, corners.map(camera_point).to_vec());
    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, vec![[0.0, 1.0, 0.0]; 4]);
    mesh.insert_attribute(
        Mesh::ATTRIBUTE_UV_0,
        vec![[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]],
    );
    mesh.insert_indices(Indices::U32(vec![0, 1, 2, 0, 2, 3]));
    mesh
}
fn camera_viewport_corners(camera_matrix: &CameraMatrix, depth: f32) -> [[f32; 3]; 4] {
    let width = camera_matrix.image_size.x().max(1.0);
    let height = camera_matrix.image_size.y().max(1.0);
    let fx = camera_matrix.intrinsics.focals.x.max(f32::EPSILON);
    let fy = camera_matrix.intrinsics.focals.y.max(f32::EPSILON);
    let cx = camera_matrix.intrinsics.optical_center.x();
    let cy = camera_matrix.intrinsics.optical_center.y();

    [
        camera_viewport_corner(0.0, 0.0, depth, fx, fy, cx, cy),
        camera_viewport_corner(width, 0.0, depth, fx, fy, cx, cy),
        camera_viewport_corner(width, height, depth, fx, fy, cx, cy),
        camera_viewport_corner(0.0, height, depth, fx, fy, cx, cy),
    ]
}

fn camera_viewport_corner(
    pixel_x: f32,
    pixel_y: f32,
    depth: f32,
    focal_x: f32,
    focal_y: f32,
    center_x: f32,
    center_y: f32,
) -> [f32; 3] {
    [
        (pixel_x - center_x) / focal_x * depth,
        (pixel_y - center_y) / focal_y * depth,
        depth,
    ]
}

fn camera_point(point: [f32; 3]) -> [f32; 3] {
    convert_point(point).to_array()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn viewport_meshes_only_rebuild_for_projection_changes() {
        let mut app = App::new();
        app.init_resource::<Assets<Mesh>>()
            .init_resource::<Assets<StandardMaterial>>()
            .init_resource::<Assets<Image>>()
            .insert_resource(Settings::default())
            .insert_resource(ViewerData {
                camera_matrix: Some(CameraMatrix::default()),
                ..default()
            })
            .add_systems(Startup, setup)
            .add_systems(Update, update_camera_viewport);
        app.update();
        let handles: Vec<_> = app
            .world_mut()
            .query::<&Mesh3d>()
            .iter(app.world())
            .map(|mesh| mesh.id())
            .collect();
        for handle in &handles {
            app.world_mut()
                .resource_mut::<Assets<Mesh>>()
                .get_mut(*handle)
                .unwrap()
                .insert_attribute(Mesh::ATTRIBUTE_COLOR, vec![[1.0; 4]; 4]);
        }
        app.world_mut()
            .resource_mut::<ViewerData>()
            .camera_matrix
            .as_mut()
            .unwrap()
            .robot_to_head
            .inner
            .translation
            .vector
            .x += 1.0;
        app.update();
        for handle in &handles {
            assert!(
                app.world()
                    .resource::<Assets<Mesh>>()
                    .get(*handle)
                    .unwrap()
                    .contains_attribute(Mesh::ATTRIBUTE_COLOR)
            );
        }
        app.world_mut()
            .resource_mut::<ViewerData>()
            .camera_matrix
            .as_mut()
            .unwrap()
            .intrinsics
            .focals
            .x += 1.0;
        app.update();
        for handle in &handles {
            assert!(
                !app.world()
                    .resource::<Assets<Mesh>>()
                    .get(*handle)
                    .unwrap()
                    .contains_attribute(Mesh::ATTRIBUTE_COLOR)
            );
        }
    }
}
