use std::{collections::HashMap, path::Path};

use bevy::{
    asset::RenderAssetUsages, camera::visibility::RenderLayers, light::NotShadowCaster,
    mesh::PrimitiveTopology, prelude::*,
};
use mujoco_rs::prelude::{MjData, MjSpec, MjtObj};

use super::object::{ObjectKind, ObjectPart};
use crate::bevy_mujoco::{MjcfObject, MujocoBody, from_mujoco};

const ROBOT_MJCF: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/assets/k1_robot.xml");
const MESH_DIRECTORY: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/assets/meshes");

#[derive(Clone, Copy)]
enum RobotMaterial {
    Silver,
    Black,
    Metal,
    Logo,
}

struct Link {
    body: &'static str,
    mesh: &'static str,
    material: RobotMaterial,
}

const LINKS: &[Link] = &[
    Link {
        body: "Trunk",
        mesh: "Trunk.STL",
        material: RobotMaterial::Silver,
    },
    Link {
        body: "Trunk",
        mesh: "K1logo.STL",
        material: RobotMaterial::Logo,
    },
    Link {
        body: "Head_1",
        mesh: "Head_1.STL",
        material: RobotMaterial::Black,
    },
    Link {
        body: "Head_2",
        mesh: "Head_2.STL",
        material: RobotMaterial::Black,
    },
    Link {
        body: "Left_Arm_1",
        mesh: "Left_Arm_1.STL",
        material: RobotMaterial::Metal,
    },
    Link {
        body: "Left_Arm_2",
        mesh: "Left_Arm_2.STL",
        material: RobotMaterial::Metal,
    },
    Link {
        body: "Left_Arm_3",
        mesh: "Left_Arm_3.STL",
        material: RobotMaterial::Metal,
    },
    Link {
        body: "left_hand_link",
        mesh: "Left_Arm_4.STL",
        material: RobotMaterial::Metal,
    },
    Link {
        body: "Right_Arm_1",
        mesh: "Right_Arm_1.STL",
        material: RobotMaterial::Metal,
    },
    Link {
        body: "Right_Arm_2",
        mesh: "Right_Arm_2.STL",
        material: RobotMaterial::Metal,
    },
    Link {
        body: "Right_Arm_3",
        mesh: "Right_Arm_3.STL",
        material: RobotMaterial::Metal,
    },
    Link {
        body: "right_hand_link",
        mesh: "Right_Arm_4.STL",
        material: RobotMaterial::Metal,
    },
    Link {
        body: "Left_Hip_Pitch",
        mesh: "Left_Hip_Pitch.STL",
        material: RobotMaterial::Metal,
    },
    Link {
        body: "Left_Hip_Roll",
        mesh: "Left_Hip_Roll.STL",
        material: RobotMaterial::Metal,
    },
    Link {
        body: "Left_Hip_Yaw",
        mesh: "Left_Hip_Yaw.STL",
        material: RobotMaterial::Metal,
    },
    Link {
        body: "Left_Shank",
        mesh: "Left_Shank.STL",
        material: RobotMaterial::Black,
    },
    Link {
        body: "Left_Ankle_Cross",
        mesh: "Left_Ankle_Cross.STL",
        material: RobotMaterial::Black,
    },
    Link {
        body: "left_foot_link",
        mesh: "Left_Foot.STL",
        material: RobotMaterial::Silver,
    },
    Link {
        body: "Right_Hip_Pitch",
        mesh: "Right_Hip_Pitch.STL",
        material: RobotMaterial::Metal,
    },
    Link {
        body: "Right_Hip_Roll",
        mesh: "Right_Hip_Roll.STL",
        material: RobotMaterial::Metal,
    },
    Link {
        body: "Right_Hip_Yaw",
        mesh: "Right_Hip_Yaw.STL",
        material: RobotMaterial::Metal,
    },
    Link {
        body: "Right_Shank",
        mesh: "Right_Shank.STL",
        material: RobotMaterial::Black,
    },
    Link {
        body: "Right_Ankle_Cross",
        mesh: "Right_Ankle_Cross.STL",
        material: RobotMaterial::Black,
    },
    Link {
        body: "right_foot_link",
        mesh: "Right_Foot.STL",
        material: RobotMaterial::Silver,
    },
];

#[derive(Resource)]
pub struct RobotAssets {
    meshes: HashMap<&'static str, Handle<Mesh>>,
    silver: Handle<StandardMaterial>,
    black: Handle<StandardMaterial>,
    metal: Handle<StandardMaterial>,
    logo: Handle<StandardMaterial>,
    ghost_silver: Handle<StandardMaterial>,
    ghost_black: Handle<StandardMaterial>,
    ghost_metal: Handle<StandardMaterial>,
    ghost_logo: Handle<StandardMaterial>,
    rest_poses: HashMap<&'static str, Transform>,
    ground_offset: f32,
}

impl FromWorld for RobotAssets {
    fn from_world(world: &mut World) -> Self {
        let mut spec = MjSpec::from_xml(ROBOT_MJCF).expect("robot MJCF should parse");
        let model = spec.compile().expect("robot MJCF should compile");
        let mut data = MjData::new(Box::new(model));
        data.forward();
        let root_pose = body_pose(&data, "Trunk");
        let rest_poses = LINKS
            .iter()
            .map(|link| {
                let pose = body_pose(&data, link.body);
                (
                    link.body,
                    Transform::from_matrix(root_pose.to_matrix().inverse() * pose.to_matrix()),
                )
            })
            .collect();
        let ground_offset = robot_ground_offset(&data);
        let meshes = LINKS
            .iter()
            .map(|link| {
                let path = Path::new(MESH_DIRECTORY).join(link.mesh);
                let mesh = load_binary_stl(&path)
                    .unwrap_or_else(|error| panic!("failed to load {}: {error}", path.display()));
                (link.mesh, world.resource_mut::<Assets<Mesh>>().add(mesh))
            })
            .collect();
        let mut materials = world.resource_mut::<Assets<StandardMaterial>>();
        let material = |color: Color,
                        metallic: f32,
                        roughness: f32,
                        alpha: f32,
                        materials: &mut Assets<StandardMaterial>| {
            materials.add(StandardMaterial {
                base_color: color.with_alpha(alpha),
                metallic,
                perceptual_roughness: roughness,
                alpha_mode: if alpha < 1.0 {
                    AlphaMode::Blend
                } else {
                    AlphaMode::Opaque
                },
                ..default()
            })
        };

        Self {
            meshes,
            silver: material(Color::srgb(0.8, 0.8, 0.8), 0.0, 0.5, 1.0, &mut materials),
            black: material(Color::srgb(0.1, 0.1, 0.1), 0.0, 0.5, 1.0, &mut materials),
            metal: material(Color::srgb(0.1, 0.1, 0.1), 0.1, 0.9, 1.0, &mut materials),
            logo: material(
                Color::srgb(0.792_156_9, 0.819_607_85, 0.933_333_34),
                0.0,
                0.5,
                1.0,
                &mut materials,
            ),
            ghost_silver: material(Color::srgb(0.8, 0.8, 0.8), 0.0, 0.5, 0.38, &mut materials),
            ghost_black: material(Color::srgb(0.1, 0.1, 0.1), 0.0, 0.5, 0.38, &mut materials),
            ghost_metal: material(Color::srgb(0.1, 0.1, 0.1), 0.1, 0.9, 0.38, &mut materials),
            ghost_logo: material(
                Color::srgb(0.792_156_9, 0.819_607_85, 0.933_333_34),
                0.0,
                0.5,
                0.38,
                &mut materials,
            ),
            rest_poses,
            ground_offset,
        }
    }
}

impl RobotAssets {
    fn material(&self, material: RobotMaterial, ghost: bool) -> Handle<StandardMaterial> {
        match (material, ghost) {
            (RobotMaterial::Silver, false) => self.silver.clone(),
            (RobotMaterial::Black, false) => self.black.clone(),
            (RobotMaterial::Metal, false) => self.metal.clone(),
            (RobotMaterial::Logo, false) => self.logo.clone(),
            (RobotMaterial::Silver, true) => self.ghost_silver.clone(),
            (RobotMaterial::Black, true) => self.ghost_black.clone(),
            (RobotMaterial::Metal, true) => self.ghost_metal.clone(),
            (RobotMaterial::Logo, true) => self.ghost_logo.clone(),
        }
    }

    pub fn ground_offset(&self) -> f32 {
        self.ground_offset
    }

    pub fn spawn_visual(
        &self,
        commands: &mut Commands,
        transform: Transform,
        ghost: bool,
        layers: RenderLayers,
    ) -> Entity {
        let root = commands
            .spawn((
                transform,
                Visibility::default(),
                layers.clone(),
                Pickable::IGNORE,
            ))
            .id();
        commands.entity(root).with_children(|parent| {
            for link in LINKS {
                let mut visual = parent.spawn((
                    Mesh3d(self.meshes[link.mesh].clone()),
                    MeshMaterial3d(self.material(link.material, ghost)),
                    self.rest_poses[link.body],
                    layers.clone(),
                    Pickable::IGNORE,
                ));
                if ghost {
                    visual.insert(NotShadowCaster);
                }
            }
        });
        root
    }
}

pub fn spawn(commands: &mut Commands, assets: &RobotAssets, transform: Transform) -> Entity {
    let owner = commands
        .spawn((
            ObjectKind::Robot,
            MjcfObject::new(ROBOT_MJCF, "Trunk")
                .with_free_joint("world_joint")
                .grounded(),
            transform,
        ))
        .id();

    for link in LINKS {
        commands.spawn((
            Name::new(link.mesh),
            ObjectPart(owner),
            MujocoBody::new(owner, link.body),
            Mesh3d(assets.meshes[link.mesh].clone()),
            MeshMaterial3d(assets.material(link.material, false)),
            Transform::default(),
            Visibility::Hidden,
        ));
    }

    owner
}

fn body_pose(data: &MjData<Box<mujoco_rs::prelude::MjModel>>, name: &str) -> Transform {
    let body = data.body(name).expect("robot body should exist").view(data);
    from_mujoco(
        [body.xpos[0], body.xpos[1], body.xpos[2]],
        [body.xquat[0], body.xquat[1], body.xquat[2], body.xquat[3]],
    )
}

fn robot_ground_offset(data: &MjData<Box<mujoco_rs::prelude::MjModel>>) -> f32 {
    let model = data.model();
    let root = model
        .name_to_id(MjtObj::mjOBJ_BODY, "Trunk")
        .expect("robot root should exist");
    let minimum = model
        .geom_bodyid()
        .iter()
        .enumerate()
        .filter_map(|(geom, &body)| {
            let mut body = usize::try_from(body).ok()?;
            while body != root {
                let parent = usize::try_from(model.body_parentid()[body]).ok()?;
                if parent == body {
                    return None;
                }
                body = parent;
            }
            let [cx, cy, cz, hx, hy, hz] = model.geom_aabb()[geom];
            let [_, _, _, _, _, _, r20, r21, r22] = data.geom_xmat()[geom];
            Some(
                data.geom_xpos()[geom][2] + r20 * cx + r21 * cy + r22 * cz
                    - r20.abs() * hx
                    - r21.abs() * hy
                    - r22.abs() * hz,
            )
        })
        .reduce(f64::min)
        .expect("robot should have geometry");
    let root_z = data.body("Trunk").unwrap().view(data).xpos[2];
    (root_z - minimum) as f32
}

fn load_binary_stl(path: &Path) -> Result<Mesh, String> {
    let bytes = std::fs::read(path).map_err(|error| error.to_string())?;
    if bytes.len() < 84 {
        return Err("file is too short to be a binary STL".to_owned());
    }

    let triangle_count =
        u32::from_le_bytes(bytes[80..84].try_into().expect("slice has length 4")) as usize;
    let expected_length = 84 + triangle_count * 50;
    if bytes.len() < expected_length {
        return Err(format!(
            "expected {expected_length} bytes, got {}",
            bytes.len()
        ));
    }

    let mut positions = Vec::with_capacity(triangle_count * 3);
    let mut normals = Vec::with_capacity(triangle_count * 3);
    let mut offset = 84;

    for _ in 0..triangle_count {
        let normal = convert(read_vec3(&bytes, offset));
        offset += 12;
        let mut triangle = [Vec3::ZERO; 3];
        for vertex in &mut triangle {
            *vertex = convert(read_vec3(&bytes, offset));
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
    }

    Ok(Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::RENDER_WORLD,
    )
    .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, positions)
    .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, normals))
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

fn convert([x, y, z]: [f32; 3]) -> Vec3 {
    Vec3::new(x, z, -y)
}
