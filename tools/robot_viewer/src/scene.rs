use std::{path::Path, sync::Arc};

use bevy::{
    asset::RenderAssetUsages,
    mesh::{Indices, PrimitiveTopology},
    prelude::*,
    render::render_resource::{Extent3d, TextureDimension, TextureFormat},
};
use coordinate_systems::{Field, Robot};
use field_mark_association::FieldMarkAssociations;
use kinematics::robot_kinematics::RobotKinematics;
use linear_algebra::Isometry3;
use projection::{Projection, camera_matrix::CameraMatrix};
use types::field_dimensions::FieldDimensions;

use crate::state::{AlignedViewerState, CameraFrame, PoseSource};

const K1_ASSET_DIRECTORY: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../mujoco-simulator/mujoco-simulator/K1"
);
const CAMERA_VIEWPORT_DEPTH: f32 = 1.0;
pub(crate) fn configure(app: &mut App) {
    app.insert_resource(ViewerData::default())
        .insert_resource(GlobalAmbientLight {
            color: Color::WHITE,
            brightness: 600.0,
            ..default()
        })
        .add_systems(Startup, setup_scene)
        .add_systems(
            Update,
            (
                position_camera_once,
                update_field_plane,
                update_field_markings,
                update_camera_viewport,
                update_camera_image,
                update_field_mark_associations,
                update_robot_links,
            ),
        );
}

#[derive(Clone, Default, Resource)]
pub(crate) struct ViewerData {
    pose_source: PoseSource,
    field_dimensions: Option<FieldDimensions>,
    localization: Option<Isometry3<Field, Robot>>,
    visual_odometer: Option<nalgebra::Isometry3<f32>>,
    robot_kinematics: Option<Arc<RobotKinematics>>,
    camera_matrix: Option<CameraMatrix>,
    camera_frame: Option<Arc<CameraFrame>>,
    field_mark_associations: Option<Arc<FieldMarkAssociations>>,
}

impl ViewerData {
    pub(crate) fn from_aligned_state(state: AlignedViewerState, pose_source: PoseSource) -> Self {
        let camera_matrix = state.camera_matrix.map(|sample| {
            let mut camera_matrix = sample.inner.as_ref().clone();
            if let Some(intrinsics) = state.latest_calibrated_intrinsics {
                camera_matrix.intrinsics = intrinsics;
            }
            camera_matrix
        });

        Self {
            pose_source,
            field_dimensions: state.field_dimensions,
            localization: state.latest_localization,
            visual_odometer: state.latest_visual_odometer,
            robot_kinematics: state.robot_kinematics.map(|sample| sample.inner),
            camera_matrix,
            camera_frame: state.camera_frame.map(|sample| sample.inner),
            field_mark_associations: state.field_mark_associations.map(|sample| sample.inner),
        }
    }
}

#[derive(Component)]
struct FieldPlane;

#[derive(Component)]
struct FieldMarkings;

#[derive(Component)]
struct CameraFrustum;

#[derive(Component)]
struct CameraImagePlane {
    texture: Handle<Image>,
    sequence: u64,
}

#[derive(Component)]
struct FieldMarkAssociationLines;

#[derive(Component)]
struct RobotLink {
    frame: RobotFrame,
    fallback_translation: [f32; 3],
}

#[derive(Clone, Copy)]
enum RobotFrame {
    Torso,
    Neck,
    Head,
    LeftInnerShoulder,
    LeftOuterShoulder,
    LeftUpperArm,
    LeftForearm,
    RightInnerShoulder,
    RightOuterShoulder,
    RightUpperArm,
    RightForearm,
    LeftPelvis,
    LeftHip,
    LeftThigh,
    LeftTibia,
    LeftAnkle,
    LeftFoot,
    RightPelvis,
    RightHip,
    RightThigh,
    RightTibia,
    RightAnkle,
    RightFoot,
}

impl RobotFrame {
    fn isometry(self, kinematics: &RobotKinematics) -> nalgebra::Isometry3<f32> {
        match self {
            Self::Torso => kinematics.torso.torso_to_robot.inner,
            Self::Neck => kinematics.head.neck_to_robot.inner,
            Self::Head => kinematics.head.head_to_robot.inner,
            Self::LeftInnerShoulder => kinematics.left_arm.inner_shoulder_to_robot.inner,
            Self::LeftOuterShoulder => kinematics.left_arm.outer_shoulder_to_robot.inner,
            Self::LeftUpperArm => kinematics.left_arm.upper_arm_to_robot.inner,
            Self::LeftForearm => kinematics.left_arm.forearm_to_robot.inner,
            Self::RightInnerShoulder => kinematics.right_arm.inner_shoulder_to_robot.inner,
            Self::RightOuterShoulder => kinematics.right_arm.outer_shoulder_to_robot.inner,
            Self::RightUpperArm => kinematics.right_arm.upper_arm_to_robot.inner,
            Self::RightForearm => kinematics.right_arm.forearm_to_robot.inner,
            Self::LeftPelvis => kinematics.left_leg.pelvis_to_robot.inner,
            Self::LeftHip => kinematics.left_leg.hip_to_robot.inner,
            Self::LeftThigh => kinematics.left_leg.thigh_to_robot.inner,
            Self::LeftTibia => kinematics.left_leg.tibia_to_robot.inner,
            Self::LeftAnkle => kinematics.left_leg.ankle_to_robot.inner,
            Self::LeftFoot => kinematics.left_leg.foot_to_robot.inner,
            Self::RightPelvis => kinematics.right_leg.pelvis_to_robot.inner,
            Self::RightHip => kinematics.right_leg.hip_to_robot.inner,
            Self::RightThigh => kinematics.right_leg.thigh_to_robot.inner,
            Self::RightTibia => kinematics.right_leg.tibia_to_robot.inner,
            Self::RightAnkle => kinematics.right_leg.ankle_to_robot.inner,
            Self::RightFoot => kinematics.right_leg.foot_to_robot.inner,
        }
    }
}

#[derive(Clone, Copy)]
enum RobotMaterial {
    SilverPlastic,
    BlackPlastic,
    BlackMetalRough,
    Logo,
}

impl RobotMaterial {
    fn material(self, materials: &mut Assets<StandardMaterial>) -> Handle<StandardMaterial> {
        let (color, metallic, roughness, reflectance) = match self {
            Self::SilverPlastic => (Color::srgba(0.8, 0.8, 0.8, 1.0), 0.0, 0.5, 0.0),
            Self::BlackPlastic => (Color::srgba(0.1, 0.1, 0.1, 1.0), 0.0, 0.5, 0.0),
            Self::BlackMetalRough => (Color::srgba(0.1, 0.1, 0.1, 1.0), 0.1, 0.9, 0.1),
            Self::Logo => (
                Color::srgba(0.792_156_9, 0.819_607_85, 0.933_333_34, 1.0),
                0.0,
                0.5,
                0.0,
            ),
        };

        materials.add(StandardMaterial {
            base_color: color,
            metallic,
            perceptual_roughness: roughness,
            reflectance,
            ..default()
        })
    }
}

struct LinkDescriptor {
    name: &'static str,
    mesh: &'static str,
    material: RobotMaterial,
    frame: RobotFrame,
    fallback_translation: [f32; 3],
}

const LINK_DESCRIPTORS: &[LinkDescriptor] = &[
    LinkDescriptor {
        name: "Trunk",
        mesh: "Trunk.STL",
        material: RobotMaterial::SilverPlastic,
        frame: RobotFrame::Torso,
        fallback_translation: [0.0, 0.0, 0.6],
    },
    LinkDescriptor {
        name: "K1logo",
        mesh: "K1logo.STL",
        material: RobotMaterial::Logo,
        frame: RobotFrame::Torso,
        fallback_translation: [0.0, 0.0, 0.6],
    },
    LinkDescriptor {
        name: "Head_1",
        mesh: "Head_1.STL",
        material: RobotMaterial::BlackPlastic,
        frame: RobotFrame::Neck,
        fallback_translation: [0.0056, 0.0, 0.8149],
    },
    LinkDescriptor {
        name: "Head_2",
        mesh: "Head_2.STL",
        material: RobotMaterial::BlackPlastic,
        frame: RobotFrame::Head,
        fallback_translation: [0.0056, 0.0, 0.8479],
    },
    LinkDescriptor {
        name: "Left_Arm_1",
        mesh: "Left_Arm_1.STL",
        material: RobotMaterial::BlackMetalRough,
        frame: RobotFrame::LeftInnerShoulder,
        fallback_translation: [0.0, 0.077, 0.7845],
    },
    LinkDescriptor {
        name: "Left_Arm_2",
        mesh: "Left_Arm_2.STL",
        material: RobotMaterial::BlackMetalRough,
        frame: RobotFrame::LeftOuterShoulder,
        fallback_translation: [0.0025, 0.145, 0.771],
    },
    LinkDescriptor {
        name: "Left_Arm_3",
        mesh: "Left_Arm_3.STL",
        material: RobotMaterial::BlackMetalRough,
        frame: RobotFrame::LeftUpperArm,
        fallback_translation: [0.0025, 0.189_428, 0.771],
    },
    LinkDescriptor {
        name: "Left_Arm_4",
        mesh: "Left_Arm_4.STL",
        material: RobotMaterial::BlackMetalRough,
        frame: RobotFrame::LeftForearm,
        fallback_translation: [0.0025, 0.310_928, 0.771],
    },
    LinkDescriptor {
        name: "Right_Arm_1",
        mesh: "Right_Arm_1.STL",
        material: RobotMaterial::BlackMetalRough,
        frame: RobotFrame::RightInnerShoulder,
        fallback_translation: [0.0, -0.077, 0.7845],
    },
    LinkDescriptor {
        name: "Right_Arm_2",
        mesh: "Right_Arm_2.STL",
        material: RobotMaterial::BlackMetalRough,
        frame: RobotFrame::RightOuterShoulder,
        fallback_translation: [0.0025, -0.145, 0.771],
    },
    LinkDescriptor {
        name: "Right_Arm_3",
        mesh: "Right_Arm_3.STL",
        material: RobotMaterial::BlackMetalRough,
        frame: RobotFrame::RightUpperArm,
        fallback_translation: [0.0025, -0.189_428, 0.771],
    },
    LinkDescriptor {
        name: "Right_Arm_4",
        mesh: "Right_Arm_4.STL",
        material: RobotMaterial::BlackMetalRough,
        frame: RobotFrame::RightForearm,
        fallback_translation: [0.0025, -0.310_928, 0.771],
    },
    LinkDescriptor {
        name: "Left_Hip_Pitch",
        mesh: "Left_Hip_Pitch.STL",
        material: RobotMaterial::BlackMetalRough,
        frame: RobotFrame::LeftPelvis,
        fallback_translation: [0.0, 0.096, 0.523],
    },
    LinkDescriptor {
        name: "Left_Hip_Roll",
        mesh: "Left_Hip_Roll.STL",
        material: RobotMaterial::BlackMetalRough,
        frame: RobotFrame::LeftHip,
        fallback_translation: [0.0, 0.096, 0.497],
    },
    LinkDescriptor {
        name: "Left_Hip_Yaw",
        mesh: "Left_Hip_Yaw.STL",
        material: RobotMaterial::BlackMetalRough,
        frame: RobotFrame::LeftThigh,
        fallback_translation: [0.012, 0.096, 0.4485],
    },
    LinkDescriptor {
        name: "Left_Shank",
        mesh: "Left_Shank.STL",
        material: RobotMaterial::BlackPlastic,
        frame: RobotFrame::LeftTibia,
        fallback_translation: [-0.002, 0.096, 0.3315],
    },
    LinkDescriptor {
        name: "Left_Ankle_Cross",
        mesh: "Left_Ankle_Cross.STL",
        material: RobotMaterial::BlackPlastic,
        frame: RobotFrame::LeftAnkle,
        fallback_translation: [-0.001_802_94, 0.0962, 0.08631],
    },
    LinkDescriptor {
        name: "Left_Foot",
        mesh: "Left_Foot.STL",
        material: RobotMaterial::SilverPlastic,
        frame: RobotFrame::LeftFoot,
        fallback_translation: [-0.001_802_94, 0.0962, 0.08631],
    },
    LinkDescriptor {
        name: "Right_Hip_Pitch",
        mesh: "Right_Hip_Pitch.STL",
        material: RobotMaterial::BlackMetalRough,
        frame: RobotFrame::RightPelvis,
        fallback_translation: [0.0, -0.096, 0.523],
    },
    LinkDescriptor {
        name: "Right_Hip_Roll",
        mesh: "Right_Hip_Roll.STL",
        material: RobotMaterial::BlackMetalRough,
        frame: RobotFrame::RightHip,
        fallback_translation: [0.0, -0.096, 0.497],
    },
    LinkDescriptor {
        name: "Right_Hip_Yaw",
        mesh: "Right_Hip_Yaw.STL",
        material: RobotMaterial::BlackMetalRough,
        frame: RobotFrame::RightThigh,
        fallback_translation: [0.012, -0.096, 0.4485],
    },
    LinkDescriptor {
        name: "Right_Shank",
        mesh: "Right_Shank.STL",
        material: RobotMaterial::BlackPlastic,
        frame: RobotFrame::RightTibia,
        fallback_translation: [-0.002, -0.096, 0.3315],
    },
    LinkDescriptor {
        name: "Right_Ankle_Cross",
        mesh: "Right_Ankle_Cross.STL",
        material: RobotMaterial::BlackPlastic,
        frame: RobotFrame::RightAnkle,
        fallback_translation: [-0.001_802_94, -0.0962, 0.08631],
    },
    LinkDescriptor {
        name: "Right_Foot",
        mesh: "Right_Foot.STL",
        material: RobotMaterial::SilverPlastic,
        frame: RobotFrame::RightFoot,
        fallback_translation: [-0.001_802_94, -0.0962, 0.08631],
    },
];

fn setup_scene(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut images: ResMut<Assets<Image>>,
) {
    commands.spawn((
        PointLight {
            intensity: 2_500.0,
            range: 14.0,
            ..default()
        },
        Transform::from_xyz(0.0, 5.0, 0.0),
    ));

    let field_material = materials.add(StandardMaterial {
        base_color: Color::srgb(0.04, 0.34, 0.13),
        perceptual_roughness: 0.95,
        ..default()
    });
    let markings_material = materials.add(StandardMaterial {
        base_color: Color::WHITE,
        unlit: true,
        cull_mode: None,
        ..default()
    });
    let frustum_material = materials.add(StandardMaterial {
        base_color: Color::srgb(1.0, 0.85, 0.15),
        unlit: true,
        ..default()
    });
    let association_material = materials.add(StandardMaterial {
        base_color: Color::srgb(1.0, 0.1, 0.72),
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
        FieldPlane,
        Mesh3d(meshes.add(field_mesh())),
        MeshMaterial3d(field_material),
        Transform::default(),
    ));
    commands.spawn((
        FieldMarkings,
        Mesh3d(meshes.add(field_markings_mesh(&FieldDimensions::SPL_2025))),
        MeshMaterial3d(markings_material),
        Transform::default(),
    ));
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
            sequence: 0,
        },
        Mesh3d(meshes.add(empty_mesh(PrimitiveTopology::TriangleList))),
        MeshMaterial3d(camera_image_material),
        Transform::default(),
        Visibility::Hidden,
    ));
    commands.spawn((
        FieldMarkAssociationLines,
        Mesh3d(meshes.add(empty_mesh(PrimitiveTopology::LineList))),
        MeshMaterial3d(association_material),
        Transform::default(),
        Visibility::Hidden,
    ));

    for descriptor in LINK_DESCRIPTORS {
        let mesh_path = Path::new(K1_ASSET_DIRECTORY)
            .join("meshes")
            .join(descriptor.mesh);
        let mesh = match load_binary_stl(&mesh_path) {
            Ok(mesh) => mesh,
            Err(error) => {
                eprintln!("failed to load {}: {error}", mesh_path.display());
                continue;
            }
        };

        commands.spawn((
            Name::new(descriptor.name),
            RobotLink {
                frame: descriptor.frame,
                fallback_translation: descriptor.fallback_translation,
            },
            Mesh3d(meshes.add(mesh)),
            MeshMaterial3d(descriptor.material.material(&mut materials)),
            Transform::default(),
        ));
    }
}

fn position_camera_once(
    mut positioned: Local<bool>,
    mut cameras: Query<&mut Transform, With<Camera3d>>,
) {
    if *positioned {
        return;
    }

    for mut transform in &mut cameras {
        *transform = Transform::from_xyz(4.0, 6.0, 7.0).looking_at(Vec3::ZERO, Vec3::Y);
        *positioned = true;
    }
}

fn update_field_plane(data: Res<ViewerData>, mut field: Single<&mut Transform, With<FieldPlane>>) {
    let dimensions = data.field_dimensions.unwrap_or(FieldDimensions::SPL_2025);
    let length = dimensions.length + 2.0 * dimensions.border_strip_width;
    let width = dimensions.width + 2.0 * dimensions.border_strip_width;

    field.scale = Vec3::new(length, 1.0, width);
}

fn update_field_markings(
    data: Res<ViewerData>,
    markings: Single<&Mesh3d, With<FieldMarkings>>,
    mut meshes: ResMut<Assets<Mesh>>,
) {
    let dimensions = data.field_dimensions.unwrap_or(FieldDimensions::SPL_2025);
    meshes
        .insert(markings.id(), field_markings_mesh(&dimensions))
        .expect("field markings mesh handle should be valid");
}

fn update_camera_viewport(
    data: Res<ViewerData>,
    mut frustum: Single<(&Mesh3d, &mut Transform, &mut Visibility), With<CameraFrustum>>,
    mut image_plane: Single<
        (&Mesh3d, &mut Transform, &mut Visibility),
        (With<CameraImagePlane>, Without<CameraFrustum>),
    >,
    mut meshes: ResMut<Assets<Mesh>>,
) {
    let Some(camera_matrix) = &data.camera_matrix else {
        *frustum.2 = Visibility::Hidden;
        *image_plane.2 = Visibility::Hidden;
        return;
    };

    let transform = camera_to_display_transform(&data, camera_matrix);
    *frustum.1 = transform;
    *image_plane.1 = transform;
    *frustum.2 = Visibility::Visible;
    *image_plane.2 = Visibility::Visible;

    meshes
        .insert(frustum.0.id(), camera_frustum_mesh(camera_matrix))
        .expect("camera frustum mesh handle should be valid");
    meshes
        .insert(image_plane.0.id(), camera_image_plane_mesh(camera_matrix))
        .expect("camera image plane mesh handle should be valid");
}

fn update_camera_image(
    data: Res<ViewerData>,
    mut image_plane: Single<&mut CameraImagePlane>,
    mut images: ResMut<Assets<Image>>,
) {
    let Some(frame) = &data.camera_frame else {
        return;
    };
    if image_plane.sequence == frame.sequence {
        return;
    }

    images
        .insert(image_plane.texture.id(), camera_frame_image(frame.as_ref()))
        .expect("camera image texture handle should be valid");
    image_plane.sequence = frame.sequence;
}

fn update_field_mark_associations(
    data: Res<ViewerData>,
    mut lines: Single<(&Mesh3d, &mut Visibility), With<FieldMarkAssociationLines>>,
    mut meshes: ResMut<Assets<Mesh>>,
) {
    let (Some(associations), Some(camera_matrix), Some(field_to_robot)) = (
        data.field_mark_associations.as_ref(),
        data.camera_matrix.as_ref(),
        data.localization,
    ) else {
        *lines.1 = Visibility::Hidden;
        return;
    };

    if associations.associations.is_empty() {
        *lines.1 = Visibility::Hidden;
        return;
    }

    meshes
        .insert(
            lines.0.id(),
            field_mark_associations_mesh(associations.as_ref(), camera_matrix, field_to_robot),
        )
        .expect("field mark association mesh handle should be valid");
    *lines.1 = Visibility::Visible;
}

fn update_robot_links(
    data: Res<ViewerData>,
    mut links: Query<(&RobotLink, &mut Transform), Without<FieldPlane>>,
) {
    let robot_to_display = robot_to_display(&data);

    for (link, mut transform) in &mut links {
        let link_to_robot = data
            .robot_kinematics
            .as_ref()
            .map(|kinematics| link.frame.isometry(kinematics))
            .unwrap_or_else(|| {
                nalgebra::Isometry3::translation(
                    link.fallback_translation[0],
                    link.fallback_translation[1],
                    link.fallback_translation[2],
                )
            });
        *transform = transform_from_isometry(robot_to_display * link_to_robot);
    }
}

fn camera_to_display_transform(data: &ViewerData, camera_matrix: &CameraMatrix) -> Transform {
    let robot_to_display = robot_to_display(data);
    let camera_to_robot = robot_to_camera(camera_matrix).inverse();

    transform_from_isometry(robot_to_display * camera_to_robot)
}

fn robot_to_display(data: &ViewerData) -> nalgebra::Isometry3<f32> {
    match data.pose_source {
        PoseSource::Localization => data
            .localization
            .as_ref()
            .map(|field_to_robot| field_to_robot.inverse().inner),
        PoseSource::VisualOdometer => visual_odometer_robot_to_display(data),
    }
    .unwrap_or_else(nalgebra::Isometry3::identity)
}

fn visual_odometer_robot_to_display(data: &ViewerData) -> Option<nalgebra::Isometry3<f32>> {
    let visual_odometer = data.visual_odometer?;
    let camera_matrix = data.camera_matrix.as_ref()?;

    Some(visual_odometer * robot_to_camera(camera_matrix))
}

fn robot_to_camera(camera_matrix: &CameraMatrix) -> nalgebra::Isometry3<f32> {
    (camera_matrix.head_to_camera * camera_matrix.robot_to_head).inner
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

fn field_mark_associations_mesh(
    associations: &FieldMarkAssociations,
    camera_matrix: &CameraMatrix,
    field_to_robot: Isometry3<Field, Robot>,
) -> Mesh {
    let robot_to_field = field_to_robot.inverse();
    let ground_to_field = robot_to_field * camera_matrix.ground_to_robot;
    let mut positions = Vec::with_capacity(associations.associations.len() * 10);

    for association in &associations.associations {
        let Some(back_projected) = camera_matrix
            .pixel_to_ground(association.detection)
            .ok()
            .map(|ground| ground_to_field * ground.extend(0.0))
        else {
            continue;
        };
        let field_point = association.field_point;
        positions.push(field_point_position(back_projected));
        positions.push(field_point_position(field_point));
        add_field_cross(&mut positions, back_projected, 0.07);
        add_field_cross(&mut positions, field_point, 0.1);
    }

    let mut mesh = Mesh::new(PrimitiveTopology::LineList, RenderAssetUsages::RENDER_WORLD);
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
    mesh
}

fn add_field_cross(positions: &mut Vec<[f32; 3]>, point: linear_algebra::Point3<Field>, size: f32) {
    let x = point.x();
    let y = point.y();
    let z = point.z();
    positions.push(field_position(x - size, y, z));
    positions.push(field_position(x + size, y, z));
    positions.push(field_position(x, y - size, z));
    positions.push(field_position(x, y + size, z));
}

fn field_point_position(point: linear_algebra::Point3<Field>) -> [f32; 3] {
    field_position(point.x(), point.y(), point.z())
}

fn field_position(x: f32, y: f32, z: f32) -> [f32; 3] {
    [x, z + 0.08, -y]
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

fn camera_frame_image(frame: &CameraFrame) -> Image {
    Image::new(
        Extent3d {
            width: frame.width,
            height: frame.height,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        frame.rgba.clone(),
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::RENDER_WORLD,
    )
}

fn field_markings_mesh(dimensions: &FieldDimensions) -> Mesh {
    let mut mesh = FieldMarkingMesh::default();
    let line_width = dimensions.line_width.max(0.001);
    let half_length = dimensions.length / 2.0;
    let half_width = dimensions.width / 2.0;

    mesh.add_rect_stroke(
        -half_length,
        -half_width,
        half_length,
        half_width,
        line_width,
    );
    mesh.add_segment([0.0, -half_width], [0.0, half_width], line_width);
    mesh.add_arc(
        [0.0, 0.0],
        dimensions.center_circle_diameter / 2.0,
        0.0,
        std::f32::consts::TAU,
        line_width,
    );

    for sign in [-1.0, 1.0] {
        mesh.add_goal_area(
            dimensions,
            sign,
            dimensions.goal_box_area_length,
            dimensions.goal_box_area_width,
            line_width,
        );
        mesh.add_goal_area(
            dimensions,
            sign,
            dimensions.penalty_area_length,
            dimensions.penalty_area_width,
            line_width,
        );

        let penalty_x = sign * (half_length - dimensions.penalty_marker_distance);
        mesh.add_marker_cross([penalty_x, 0.0], dimensions.penalty_marker_size, line_width);

        let post_y = (dimensions.goal_inner_width + dimensions.goal_post_diameter) / 2.0;
        mesh.add_disk(
            [sign * half_length, post_y],
            dimensions.goal_post_diameter / 2.0,
        );
        mesh.add_disk(
            [sign * half_length, -post_y],
            dimensions.goal_post_diameter / 2.0,
        );

        if dimensions.corner_arc_radius > 0.0 {
            mesh.add_corner_arcs(dimensions, sign, line_width);
        }
    }

    mesh.finish()
}

#[derive(Default)]
struct FieldMarkingMesh {
    positions: Vec<[f32; 3]>,
    normals: Vec<[f32; 3]>,
    uvs: Vec<[f32; 2]>,
    indices: Vec<u32>,
}

impl FieldMarkingMesh {
    const HEIGHT: f32 = 0.025;

    fn add_goal_area(
        &mut self,
        dimensions: &FieldDimensions,
        sign: f32,
        length: f32,
        width: f32,
        line_width: f32,
    ) {
        let goal_line_x = sign * dimensions.length / 2.0;
        let inner_x = goal_line_x - sign * length;
        let half_width = width / 2.0;

        self.add_segment(
            [goal_line_x, -half_width],
            [inner_x, -half_width],
            line_width,
        );
        self.add_segment([inner_x, -half_width], [inner_x, half_width], line_width);
        self.add_segment([inner_x, half_width], [goal_line_x, half_width], line_width);
    }

    fn add_marker_cross(&mut self, center: [f32; 2], size: f32, line_width: f32) {
        let half_size = size / 2.0;
        self.add_segment(
            [center[0] - half_size, center[1]],
            [center[0] + half_size, center[1]],
            line_width,
        );
        self.add_segment(
            [center[0], center[1] - half_size],
            [center[0], center[1] + half_size],
            line_width,
        );
    }

    fn add_corner_arcs(&mut self, dimensions: &FieldDimensions, sign: f32, line_width: f32) {
        let half_length = dimensions.length / 2.0;
        let half_width = dimensions.width / 2.0;
        let radius = dimensions.corner_arc_radius;

        for side in [-1.0, 1.0] {
            let center = [sign * half_length, side * half_width];
            let start = if sign > 0.0 { 0.5 } else { 0.0 };
            let start = std::f32::consts::PI * (start + if side > 0.0 { 0.0 } else { 1.0 });
            self.add_arc(
                center,
                radius,
                start,
                start + std::f32::consts::FRAC_PI_2,
                line_width,
            );
        }
    }

    fn add_rect_stroke(&mut self, min_x: f32, min_y: f32, max_x: f32, max_y: f32, width: f32) {
        self.add_segment([min_x, min_y], [max_x, min_y], width);
        self.add_segment([max_x, min_y], [max_x, max_y], width);
        self.add_segment([max_x, max_y], [min_x, max_y], width);
        self.add_segment([min_x, max_y], [min_x, min_y], width);
    }

    fn add_segment(&mut self, start: [f32; 2], end: [f32; 2], width: f32) {
        let delta = [end[0] - start[0], end[1] - start[1]];
        let length = delta[0].hypot(delta[1]);
        if length <= f32::EPSILON {
            return;
        }

        let half_width = width / 2.0;
        let perpendicular = [
            -delta[1] / length * half_width,
            delta[0] / length * half_width,
        ];
        self.add_quad([
            [start[0] - perpendicular[0], start[1] - perpendicular[1]],
            [end[0] - perpendicular[0], end[1] - perpendicular[1]],
            [end[0] + perpendicular[0], end[1] + perpendicular[1]],
            [start[0] + perpendicular[0], start[1] + perpendicular[1]],
        ]);
    }

    fn add_arc(&mut self, center: [f32; 2], radius: f32, start: f32, end: f32, width: f32) {
        if radius <= 0.0 {
            return;
        }

        let half_width = width / 2.0;
        let inner_radius = (radius - half_width).max(0.0);
        let outer_radius = radius + half_width;
        let segments = ((radius * (end - start).abs()) / 0.05).ceil() as usize;
        let segments = segments.clamp(8, 96);

        for index in 0..segments {
            let angle0 = start + (end - start) * index as f32 / segments as f32;
            let angle1 = start + (end - start) * (index + 1) as f32 / segments as f32;
            self.add_quad([
                arc_point(center, inner_radius, angle0),
                arc_point(center, inner_radius, angle1),
                arc_point(center, outer_radius, angle1),
                arc_point(center, outer_radius, angle0),
            ]);
        }
    }

    fn add_disk(&mut self, center: [f32; 2], radius: f32) {
        if radius <= 0.0 {
            return;
        }

        let segments = 32;
        for index in 0..segments {
            let angle0 = std::f32::consts::TAU * index as f32 / segments as f32;
            let angle1 = std::f32::consts::TAU * (index + 1) as f32 / segments as f32;
            self.add_triangle([
                center,
                arc_point(center, radius, angle1),
                arc_point(center, radius, angle0),
            ]);
        }
    }

    fn add_quad(&mut self, points: [[f32; 2]; 4]) {
        let base = self.positions.len() as u32;
        for point in points {
            self.add_vertex(point);
        }
        self.indices
            .extend_from_slice(&[base, base + 1, base + 2, base, base + 2, base + 3]);
    }

    fn add_triangle(&mut self, points: [[f32; 2]; 3]) {
        let base = self.positions.len() as u32;
        for point in points {
            self.add_vertex(point);
        }
        self.indices.extend_from_slice(&[base, base + 1, base + 2]);
    }

    fn add_vertex(&mut self, point: [f32; 2]) {
        self.positions.push([point[0], Self::HEIGHT, -point[1]]);
        self.normals.push([0.0, 1.0, 0.0]);
        self.uvs.push([0.0, 0.0]);
    }

    fn finish(self) -> Mesh {
        let mut mesh = Mesh::new(
            PrimitiveTopology::TriangleList,
            RenderAssetUsages::RENDER_WORLD,
        );
        mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, self.positions);
        mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, self.normals);
        mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, self.uvs);
        mesh.insert_indices(Indices::U32(self.indices));
        mesh
    }
}

fn arc_point(center: [f32; 2], radius: f32, angle: f32) -> [f32; 2] {
    [
        center[0] + radius * angle.cos(),
        center[1] + radius * angle.sin(),
    ]
}

fn empty_mesh(topology: PrimitiveTopology) -> Mesh {
    let mut mesh = Mesh::new(topology, RenderAssetUsages::RENDER_WORLD);
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, Vec::<[f32; 3]>::new());
    mesh
}

fn field_mesh() -> Mesh {
    let mut mesh = Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::RENDER_WORLD,
    );
    mesh.insert_attribute(
        Mesh::ATTRIBUTE_POSITION,
        vec![
            [-0.5, 0.0, -0.5],
            [0.5, 0.0, -0.5],
            [0.5, 0.0, 0.5],
            [-0.5, 0.0, 0.5],
        ],
    );
    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, vec![[0.0, 1.0, 0.0]; 4]);
    mesh.insert_attribute(
        Mesh::ATTRIBUTE_UV_0,
        vec![[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]],
    );
    mesh.insert_indices(Indices::U32(vec![0, 2, 1, 0, 3, 2]));
    mesh
}

fn load_binary_stl(path: &Path) -> Result<Mesh, String> {
    let bytes = std::fs::read(path).map_err(|error| format!("{}: {error}", path.display()))?;
    if bytes.len() < 84 {
        return Err("file is too short to be a binary STL".to_string());
    }

    let triangle_count =
        u32::from_le_bytes(bytes[80..84].try_into().expect("slice has length 4")) as usize;
    let expected_len = 84 + triangle_count * 50;
    if bytes.len() < expected_len {
        return Err(format!(
            "expected at least {expected_len} bytes for {triangle_count} triangles, got {}",
            bytes.len()
        ));
    }

    let mut positions = Vec::with_capacity(triangle_count * 3);
    let mut normals = Vec::with_capacity(triangle_count * 3);
    let mut uvs = Vec::with_capacity(triangle_count * 3);
    let mut offset = 84;

    for _ in 0..triangle_count {
        let normal = convert_vector(read_vec3(&bytes, offset));
        offset += 12;

        let mut triangle = [Vec3::ZERO; 3];
        for vertex in &mut triangle {
            *vertex = convert_point(read_vec3(&bytes, offset));
            offset += 12;
        }
        offset += 2;

        let normal = normal.try_normalize().unwrap_or_else(|| {
            (triangle[1] - triangle[0])
                .cross(triangle[2] - triangle[0])
                .normalize_or_zero()
        });

        positions.extend(triangle.map(|vertex| vertex.to_array()));
        normals.extend([normal.to_array(); 3]);
        uvs.extend([[0.0, 0.0]; 3]);
    }

    let mut mesh = Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::RENDER_WORLD,
    );
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
    mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uvs);
    Ok(mesh)
}

fn read_vec3(bytes: &[u8], offset: usize) -> [f32; 3] {
    [
        read_f32(bytes, offset),
        read_f32(bytes, offset + 4),
        read_f32(bytes, offset + 8),
    ]
}

fn read_f32(bytes: &[u8], offset: usize) -> f32 {
    f32::from_le_bytes(
        bytes[offset..offset + 4]
            .try_into()
            .expect("slice has length 4"),
    )
}

fn transform_from_isometry(isometry: nalgebra::Isometry3<f32>) -> Transform {
    Transform::from_translation(convert_point(isometry.translation.vector.into()))
        .with_rotation(convert_rotation(isometry.rotation))
}

fn convert_rotation(rotation: nalgebra::UnitQuaternion<f32>) -> Quat {
    let source = rotation.to_rotation_matrix();
    let source = source.matrix();
    let source = Mat3::from_cols(
        Vec3::new(source[(0, 0)], source[(1, 0)], source[(2, 0)]),
        Vec3::new(source[(0, 1)], source[(1, 1)], source[(2, 1)]),
        Vec3::new(source[(0, 2)], source[(1, 2)], source[(2, 2)]),
    );
    let conversion = Mat3::from_cols(Vec3::X, Vec3::NEG_Z, Vec3::Y);

    Quat::from_mat3(&(conversion * source * conversion.transpose()))
}

fn convert_point([x, y, z]: [f32; 3]) -> Vec3 {
    Vec3::new(x, z, -y)
}

fn convert_vector(vector: [f32; 3]) -> Vec3 {
    convert_point(vector)
}
