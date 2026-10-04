use super::{
    RobotKinematics, Settings, ViewerData, convert_point, robot_to_display, transform_from_isometry,
};
use bevy::{asset::RenderAssetUsages, mesh::PrimitiveTopology, prelude::*};

#[derive(Component)]
pub(super) struct RobotLink {
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
    mesh: &'static [u8],
    material: RobotMaterial,
    frame: RobotFrame,
    fallback_translation: [f32; 3],
}

const LINK_DESCRIPTORS: &[LinkDescriptor] = &[
    LinkDescriptor {
        name: "Trunk",
        mesh: include_bytes!("../../../../mujoco-simulator/mujoco-simulator/K1/meshes/Trunk.STL"),
        material: RobotMaterial::SilverPlastic,
        frame: RobotFrame::Torso,
        fallback_translation: [0.0, 0.0, 0.6],
    },
    LinkDescriptor {
        name: "K1logo",
        mesh: include_bytes!("../../../../mujoco-simulator/mujoco-simulator/K1/meshes/K1logo.STL"),
        material: RobotMaterial::Logo,
        frame: RobotFrame::Torso,
        fallback_translation: [0.0, 0.0, 0.6],
    },
    LinkDescriptor {
        name: "Head_1",
        mesh: include_bytes!("../../../../mujoco-simulator/mujoco-simulator/K1/meshes/Head_1.STL"),
        material: RobotMaterial::BlackPlastic,
        frame: RobotFrame::Neck,
        fallback_translation: [0.0056, 0.0, 0.8149],
    },
    LinkDescriptor {
        name: "Head_2",
        mesh: include_bytes!("../../../../mujoco-simulator/mujoco-simulator/K1/meshes/Head_2.STL"),
        material: RobotMaterial::BlackPlastic,
        frame: RobotFrame::Head,
        fallback_translation: [0.0056, 0.0, 0.8479],
    },
    LinkDescriptor {
        name: "Left_Arm_1",
        mesh: include_bytes!(
            "../../../../mujoco-simulator/mujoco-simulator/K1/meshes/Left_Arm_1.STL"
        ),
        material: RobotMaterial::BlackMetalRough,
        frame: RobotFrame::LeftInnerShoulder,
        fallback_translation: [0.0, 0.077, 0.7845],
    },
    LinkDescriptor {
        name: "Left_Arm_2",
        mesh: include_bytes!(
            "../../../../mujoco-simulator/mujoco-simulator/K1/meshes/Left_Arm_2.STL"
        ),
        material: RobotMaterial::BlackMetalRough,
        frame: RobotFrame::LeftOuterShoulder,
        fallback_translation: [0.0025, 0.145, 0.771],
    },
    LinkDescriptor {
        name: "Left_Arm_3",
        mesh: include_bytes!(
            "../../../../mujoco-simulator/mujoco-simulator/K1/meshes/Left_Arm_3.STL"
        ),
        material: RobotMaterial::BlackMetalRough,
        frame: RobotFrame::LeftUpperArm,
        fallback_translation: [0.0025, 0.189_428, 0.771],
    },
    LinkDescriptor {
        name: "Left_Arm_4",
        mesh: include_bytes!(
            "../../../../mujoco-simulator/mujoco-simulator/K1/meshes/Left_Arm_4.STL"
        ),
        material: RobotMaterial::BlackMetalRough,
        frame: RobotFrame::LeftForearm,
        fallback_translation: [0.0025, 0.310_928, 0.771],
    },
    LinkDescriptor {
        name: "Right_Arm_1",
        mesh: include_bytes!(
            "../../../../mujoco-simulator/mujoco-simulator/K1/meshes/Right_Arm_1.STL"
        ),
        material: RobotMaterial::BlackMetalRough,
        frame: RobotFrame::RightInnerShoulder,
        fallback_translation: [0.0, -0.077, 0.7845],
    },
    LinkDescriptor {
        name: "Right_Arm_2",
        mesh: include_bytes!(
            "../../../../mujoco-simulator/mujoco-simulator/K1/meshes/Right_Arm_2.STL"
        ),
        material: RobotMaterial::BlackMetalRough,
        frame: RobotFrame::RightOuterShoulder,
        fallback_translation: [0.0025, -0.145, 0.771],
    },
    LinkDescriptor {
        name: "Right_Arm_3",
        mesh: include_bytes!(
            "../../../../mujoco-simulator/mujoco-simulator/K1/meshes/Right_Arm_3.STL"
        ),
        material: RobotMaterial::BlackMetalRough,
        frame: RobotFrame::RightUpperArm,
        fallback_translation: [0.0025, -0.189_428, 0.771],
    },
    LinkDescriptor {
        name: "Right_Arm_4",
        mesh: include_bytes!(
            "../../../../mujoco-simulator/mujoco-simulator/K1/meshes/Right_Arm_4.STL"
        ),
        material: RobotMaterial::BlackMetalRough,
        frame: RobotFrame::RightForearm,
        fallback_translation: [0.0025, -0.310_928, 0.771],
    },
    LinkDescriptor {
        name: "Left_Hip_Pitch",
        mesh: include_bytes!(
            "../../../../mujoco-simulator/mujoco-simulator/K1/meshes/Left_Hip_Pitch.STL"
        ),
        material: RobotMaterial::BlackMetalRough,
        frame: RobotFrame::LeftPelvis,
        fallback_translation: [0.0, 0.096, 0.523],
    },
    LinkDescriptor {
        name: "Left_Hip_Roll",
        mesh: include_bytes!(
            "../../../../mujoco-simulator/mujoco-simulator/K1/meshes/Left_Hip_Roll.STL"
        ),
        material: RobotMaterial::BlackMetalRough,
        frame: RobotFrame::LeftHip,
        fallback_translation: [0.0, 0.096, 0.497],
    },
    LinkDescriptor {
        name: "Left_Hip_Yaw",
        mesh: include_bytes!(
            "../../../../mujoco-simulator/mujoco-simulator/K1/meshes/Left_Hip_Yaw.STL"
        ),
        material: RobotMaterial::BlackMetalRough,
        frame: RobotFrame::LeftThigh,
        fallback_translation: [0.012, 0.096, 0.4485],
    },
    LinkDescriptor {
        name: "Left_Shank",
        mesh: include_bytes!(
            "../../../../mujoco-simulator/mujoco-simulator/K1/meshes/Left_Shank.STL"
        ),
        material: RobotMaterial::BlackPlastic,
        frame: RobotFrame::LeftTibia,
        fallback_translation: [-0.002, 0.096, 0.3315],
    },
    LinkDescriptor {
        name: "Left_Ankle_Cross",
        mesh: include_bytes!(
            "../../../../mujoco-simulator/mujoco-simulator/K1/meshes/Left_Ankle_Cross.STL"
        ),
        material: RobotMaterial::BlackPlastic,
        frame: RobotFrame::LeftAnkle,
        fallback_translation: [-0.001_802_94, 0.0962, 0.08631],
    },
    LinkDescriptor {
        name: "Left_Foot",
        mesh: include_bytes!(
            "../../../../mujoco-simulator/mujoco-simulator/K1/meshes/Left_Foot.STL"
        ),
        material: RobotMaterial::SilverPlastic,
        frame: RobotFrame::LeftFoot,
        fallback_translation: [-0.001_802_94, 0.0962, 0.08631],
    },
    LinkDescriptor {
        name: "Right_Hip_Pitch",
        mesh: include_bytes!(
            "../../../../mujoco-simulator/mujoco-simulator/K1/meshes/Right_Hip_Pitch.STL"
        ),
        material: RobotMaterial::BlackMetalRough,
        frame: RobotFrame::RightPelvis,
        fallback_translation: [0.0, -0.096, 0.523],
    },
    LinkDescriptor {
        name: "Right_Hip_Roll",
        mesh: include_bytes!(
            "../../../../mujoco-simulator/mujoco-simulator/K1/meshes/Right_Hip_Roll.STL"
        ),
        material: RobotMaterial::BlackMetalRough,
        frame: RobotFrame::RightHip,
        fallback_translation: [0.0, -0.096, 0.497],
    },
    LinkDescriptor {
        name: "Right_Hip_Yaw",
        mesh: include_bytes!(
            "../../../../mujoco-simulator/mujoco-simulator/K1/meshes/Right_Hip_Yaw.STL"
        ),
        material: RobotMaterial::BlackMetalRough,
        frame: RobotFrame::RightThigh,
        fallback_translation: [0.012, -0.096, 0.4485],
    },
    LinkDescriptor {
        name: "Right_Shank",
        mesh: include_bytes!(
            "../../../../mujoco-simulator/mujoco-simulator/K1/meshes/Right_Shank.STL"
        ),
        material: RobotMaterial::BlackPlastic,
        frame: RobotFrame::RightTibia,
        fallback_translation: [-0.002, -0.096, 0.3315],
    },
    LinkDescriptor {
        name: "Right_Ankle_Cross",
        mesh: include_bytes!(
            "../../../../mujoco-simulator/mujoco-simulator/K1/meshes/Right_Ankle_Cross.STL"
        ),
        material: RobotMaterial::BlackPlastic,
        frame: RobotFrame::RightAnkle,
        fallback_translation: [-0.001_802_94, -0.0962, 0.08631],
    },
    LinkDescriptor {
        name: "Right_Foot",
        mesh: include_bytes!(
            "../../../../mujoco-simulator/mujoco-simulator/K1/meshes/Right_Foot.STL"
        ),
        material: RobotMaterial::SilverPlastic,
        frame: RobotFrame::RightFoot,
        fallback_translation: [-0.001_802_94, -0.0962, 0.08631],
    },
];
pub(super) fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    for descriptor in LINK_DESCRIPTORS {
        let mesh = match load_binary_stl(descriptor.mesh) {
            Ok(mesh) => mesh,
            Err(error) => {
                log::warn!("failed to load {}: {error}", descriptor.name);
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
pub(super) fn update(
    data: Res<ViewerData>,
    settings: Res<Settings>,
    mut links: Query<(&RobotLink, &mut Transform, &mut Visibility)>,
) {
    let robot_to_display = robot_to_display(&data);

    for (link, mut transform, mut visibility) in &mut links {
        *visibility = if settings.robot {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
        if !settings.robot {
            continue;
        }
        let link_to_robot = data
            .robot_kinematics
            .as_ref()
            .map(|kinematics| link.frame.isometry(&kinematics.value.inner))
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
fn load_binary_stl(bytes: &[u8]) -> Result<Mesh, String> {
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
        let normal = convert_point(read_vec3(bytes, offset));
        offset += 12;

        let mut triangle = [Vec3::ZERO; 3];
        for vertex in &mut triangle {
            *vertex = convert_point(read_vec3(bytes, offset));
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn embedded_meshes_parse_and_truncated_stl_is_rejected() {
        assert_eq!(LINK_DESCRIPTORS.len(), 24);
        assert!(load_binary_stl(&[]).is_err());
        for descriptor in LINK_DESCRIPTORS {
            let mesh = load_binary_stl(descriptor.mesh)
                .unwrap_or_else(|error| panic!("{}: {error}", descriptor.name));
            let triangle_count =
                u32::from_le_bytes(descriptor.mesh[80..84].try_into().unwrap()) as usize;
            assert!(triangle_count > 0, "{}", descriptor.name);
            assert_eq!(
                mesh.count_vertices(),
                triangle_count * 3,
                "{}",
                descriptor.name
            );
            assert!(load_binary_stl(&descriptor.mesh[..84]).is_err());
        }
    }
}
