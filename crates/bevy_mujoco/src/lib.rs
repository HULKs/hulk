use std::{collections::BTreeMap, f32::consts::FRAC_PI_2, thread, time::Duration};

use bevy::{
    asset::RenderAssetUsages,
    ecs::relationship::RelatedSpawnerCommands,
    image::{ImageAddressMode, ImageSampler, ImageSamplerDescriptor},
    math::Affine2,
    mesh::{Indices, PrimitiveTopology, VertexAttributeValues},
    prelude::*,
    render::render_resource::{Extent3d, TextureDimension},
};
use futures_util::{SinkExt, StreamExt};
use log::{error, info};
use simulation_message::{
    ConnectionInfo, Geom, GeomVariant, Material, SceneDescription, SceneMesh, SceneUpdate,
    ServerMessageKind, SimulatorMessage,
};
use tokio::{select, sync::mpsc};
use tokio_tungstenite::{
    connect_async_with_config,
    tungstenite::{protocol::WebSocketConfig, Message},
};

pub struct MujocoVisualizerPlugin {
    egui_ctx: egui::Context,
}

impl MujocoVisualizerPlugin {
    pub fn new(egui_ctx: egui::Context) -> Self {
        Self { egui_ctx }
    }
}

impl Plugin for MujocoVisualizerPlugin {
    fn build(&self, app: &mut App) {
        let receiver = spawn_workers_thread(self.egui_ctx.clone());

        app.insert_resource(MujocoVisualizerData { receiver })
            .add_message::<SceneDescriptionMessage>()
            .add_message::<SceneUpdateMessage>()
            .add_systems(PreUpdate, process_simulator_messages)
            .add_systems(Update, spawn_mujoco_scene)
            .add_systems(Update, update_bodies);
    }
}

#[derive(Debug, Resource)]
struct MujocoVisualizerData {
    receiver: mpsc::Receiver<ServerMessageKind>,
}

fn spawn_workers_thread(egui_ctx: egui::Context) -> mpsc::Receiver<ServerMessageKind> {
    let (update_sender, update_receiver) = mpsc::channel(10);
    thread::spawn(|| {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("failed to build async runtime");

        rt.block_on(async move {
                loop {
                    // Increase size limit for all the texture data to fit into the websocket message
                    let config = WebSocketConfig::default().max_frame_size(Some(2_usize.pow(30)));
                    let Ok((stream, _response)) = connect_async_with_config("ws://localhost:8000/", Some(config), false)
                        .await else {
                            info!("Websocket connection failed, retrying...");
                            tokio::time::sleep(Duration::from_secs(1)).await;
                            continue;
                        };
                    info!("Websocket connected");
                    let (mut sender, mut receiver) = stream.split();
                    let initial_request = ConnectionInfo::viewer();
                    sender.send(Message::text(serde_json::to_string(&initial_request).expect("failed to serialize initial request"))).await.expect("failed send initial request");
                    loop {
                        select! {
                            maybe_message = receiver.next() => {
                                let message = match maybe_message {
                                    Some(Ok(message)) => message,
                                    Some(Err(error)) => { error!("websocket receive failed: {error}"); break; }
                                    None => { error!("socket closed?"); break; }
                                };
                                let message: SimulatorMessage<ServerMessageKind> = match message {
                                    Message::Binary(bytes) => bincode::deserialize(&bytes).expect("failed to parse bincode"),
                                    _ => continue
                                };
                                update_sender.send(message.payload).await.expect("failed to send update to UI");
                                egui_ctx.request_repaint();
                            }
                        }
                    }
                }
            });
    });

    update_receiver
}

fn process_simulator_messages(
    mut scene_description_writer: MessageWriter<SceneDescriptionMessage>,
    mut scene_update_writer: MessageWriter<SceneUpdateMessage>,
    mut data: ResMut<MujocoVisualizerData>,
) {
    while let Ok(update) = data.receiver.try_recv() {
        match update {
            ServerMessageKind::SceneUpdate(scene_update) => {
                scene_update_writer.write(SceneUpdateMessage(scene_update));
            }
            ServerMessageKind::SceneDescription(scene_description) => {
                scene_description_writer.write(SceneDescriptionMessage(scene_description));
            }
            _ => info!("Received unexpected simulator data"),
        }
    }
}

fn calculate_tangents(mesh: &SceneMesh) -> Vec<[f32; 4]> {
    let mut helper_mesh = Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::RENDER_WORLD,
    );

    helper_mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, mesh.vertices.clone());
    helper_mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, mesh.normals.clone());
    helper_mesh.insert_indices(Indices::U32(
        mesh.vertex_indices
            .iter()
            .flatten()
            .map(|index| *index as u32)
            .collect(),
    ));
    // Calculating tangents requires UV data to be present, but is actually unused.
    helper_mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, vec![[0.0, 0.0]; mesh.vertices.len()]);

    helper_mesh
        .generate_tangents()
        .expect("generating tangents should succeed");

    let tangents_attribute = helper_mesh
        .remove_attribute(Mesh::ATTRIBUTE_TANGENT)
        .expect("we calculated these earlier");
    let tangents = match tangents_attribute {
        VertexAttributeValues::Float32x4(values) => values,
        _ => panic!(
            "expected tangents to be in Float32x4 format but got {tangents_attribute:?} instead"
        ),
    };
    tangents
}

fn spawn_mujoco_scene(
    mut scene_description: MessageReader<SceneDescriptionMessage>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut images: ResMut<Assets<Image>>,
    mut commands: Commands,
) {
    let Some(SceneDescriptionMessage(scene)) = scene_description.read().last() else {
        return;
    };
    // TODO(oleflb): Add MuJoCo marker component used to cleanup before spawning

    let texture_handles: BTreeMap<_, _> = scene
        .textures
        .iter()
        .map(|(id, image)| {
            let mut image = Image::new(
                Extent3d {
                    width: image.width,
                    height: image.height,
                    depth_or_array_layers: 1,
                },
                TextureDimension::D2,
                image
                    .rgb
                    .chunks(3)
                    .flat_map(|rgb| [rgb[0], rgb[1], rgb[2], 255])
                    .collect(),
                wgpu::TextureFormat::Rgba8UnormSrgb,
                RenderAssetUsages::RENDER_WORLD,
            );
            image.sampler = ImageSampler::Descriptor(ImageSamplerDescriptor {
                address_mode_u: ImageAddressMode::Repeat,
                address_mode_v: ImageAddressMode::Repeat,
                ..default()
            });

            (*id, images.add(image))
        })
        .collect();

    let mesh_handles: BTreeMap<_, _> = scene
        .meshes
        .iter()
        .map(|(id, mesh)| {
            let mut asset = Mesh::new(
                PrimitiveTopology::TriangleList,
                RenderAssetUsages::RENDER_WORLD,
            );

            // Manually resolve indices because the bevy::Mesh type does not support separate
            // indices for vertices and uv coordinates but that's what we get from MuJoCo
            let vertices: Vec<_> = mesh
                .vertex_indices
                .iter()
                .flat_map(|[a, b, c]| [mesh.vertices[*a], mesh.vertices[*b], mesh.vertices[*c]])
                .collect();
            asset.insert_attribute(Mesh::ATTRIBUTE_POSITION, vertices);

            let uv: Vec<_> = mesh
                .uv_indices
                .iter()
                .flat_map(|[a, b, c]| {
                    // For some reason MuJoCo sometimes provides UV indices without UV coordinates
                    Some([
                        *mesh.uv_coordinates.get(*a)?,
                        *mesh.uv_coordinates.get(*b)?,
                        *mesh.uv_coordinates.get(*c)?,
                    ])
                })
                .flatten()
                .collect();
            if !mesh.uv_coordinates.is_empty() {
                asset.insert_attribute(Mesh::ATTRIBUTE_UV_0, uv);
            }

            let normals: Vec<_> = mesh
                .normal_indices
                .iter()
                .flat_map(|[a, b, c]| [mesh.normals[*a], mesh.normals[*b], mesh.normals[*c]])
                .collect();
            asset.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);

            let tangents = calculate_tangents(mesh);
            let tangents: Vec<_> = mesh
                .vertex_indices
                .iter()
                .flat_map(|[a, b, c]| [tangents[*a], tangents[*b], tangents[*c]])
                .collect();
            asset.insert_attribute(Mesh::ATTRIBUTE_TANGENT, tangents);

            (*id, meshes.add(asset))
        })
        .collect();

    let material_handles: BTreeMap<_, _> = scene
        .materials
        .iter()
        .map(|(id, material)| {
            let [r, g, b, a] = material.rgba;
            let mut bevy_material = StandardMaterial::from(Color::srgba_u8(
                (r * 255.0) as u8,
                (g * 255.0) as u8,
                (b * 255.0) as u8,
                (a * 255.0) as u8,
            ));
            bevy_material.reflectance = material.specular;
            bevy_material.metallic = material.reflectance;
            bevy_material.perceptual_roughness = 1.0 - material.shininess;
            // These indices correspond to the RGB and Normal textures
            // See https://mujoco.readthedocs.io/en/latest/APIreference/APItypes.html#mjttexturerole
            bevy_material.base_color_texture = material.textures[1]
                .as_ref()
                .map(|id| texture_handles[id].clone());
            bevy_material.normal_map_texture = material.textures[5]
                .as_ref()
                .map(|id| texture_handles[id].clone());
            bevy_material.uv_transform = Affine2::from_scale(material.texrepeat.into());

            (*id, materials.add(bevy_material))
        })
        .collect();

    let scene_root = commands
        .spawn((
            SceneRootMarker,
            Visibility::default(),
            Transform::from_rotation(Quat::from_rotation_x(-FRAC_PI_2)),
        ))
        .id();

    for body in scene.bodies.values() {
        let mut parent = commands.spawn((
            Transform::default(),
            Visibility::default(),
            BodyComponent { id: body.id },
        ));
        parent.set_parent_in_place(scene_root);
        parent.with_children(|parent| {
            for geom in body.geoms.iter().map(|index| &scene.geoms[index]) {
                spawn_geom(
                    parent,
                    &mut materials,
                    &mut meshes,
                    &mesh_handles,
                    &material_handles,
                    geom,
                );
            }
        });
        if body.parent.is_none() && body.name.as_ref().is_some_and(|name| name == "Trunk") {
            parent.insert(TrunkComponent);
        }
    }
}

fn spawn_geom(
    entity_commands: &mut RelatedSpawnerCommands<'_, ChildOf>,
    materials: &mut Assets<StandardMaterial>,
    meshes: &mut Assets<Mesh>,
    mesh_handles: &BTreeMap<usize, Handle<Mesh>>,
    material_handles: &BTreeMap<usize, Handle<StandardMaterial>>,
    geom: &Geom,
) {
    let material = match geom.material {
        Material::Rgba { rgba: [r, g, b, a] } => materials.add(Color::srgba(r, g, b, a)),
        Material::Pbr { material_index } => material_handles[&material_index].clone(),
    };

    let (mesh_handle, alignment_rotation) = match geom.geom_variant {
        GeomVariant::Mesh { mesh_index } => (mesh_handles[&mesh_index].clone(), Quat::IDENTITY),
        GeomVariant::Sphere { radius } => (meshes.add(Sphere::new(radius)), Quat::IDENTITY),
        GeomVariant::Box {
            extent: [hx, hy, hz],
        } => (
            // MuJoCo box extent is half-lengths, whereas bevy cuboid takes full lengths
            meshes.add(Cuboid::new(2. * hx, 2. * hy, 2. * hz)),
            Quat::IDENTITY,
        ),
        GeomVariant::Plane {
            normal: [nx, ny, nz],
        } => {
            const SCALE: f32 = 100.0;
            let mut mesh = Plane3d::new(Vec3::new(nx, ny, nz), Vec2::splat(SCALE))
                .mesh()
                .build();

            let uv = mesh
                .attribute_mut(Mesh::ATTRIBUTE_UV_0)
                .expect("Plane3d should generate UV attributes");

            match uv {
                VertexAttributeValues::Float32x2(items) => {
                    for item in items {
                        item[0] *= SCALE;
                        item[1] *= SCALE;
                    }
                }
                _ => panic!("expected UV coordinates to be Float32x2"),
            }
            (meshes.add(mesh), Quat::IDENTITY)
        }
        GeomVariant::Cylinder {
            radius,
            half_height,
        } => (
            meshes.add(Cylinder::new(radius, 2. * half_height)),
            Quat::from_rotation_x(FRAC_PI_2),
        ),
        GeomVariant::Capsule {
            radius,
            half_height,
        } => (
            meshes.add(Capsule3d::new(radius, 2. * half_height)),
            Quat::from_rotation_x(FRAC_PI_2),
        ),
    };

    entity_commands
        .spawn((
            Transform::from_translation(Vec3::from(geom.pos)).with_rotation(bevy_quat(geom.quat)),
            InheritedVisibility::default(),
        ))
        .with_children(|parent| {
            parent.spawn((
                InheritedVisibility::default(),
                Mesh3d(mesh_handle),
                MeshMaterial3d(material),
                Transform::from_rotation(alignment_rotation),
            ));
        });
}

fn update_bodies(
    mut scene_update: MessageReader<SceneUpdateMessage>,
    mut query: Query<(&mut Transform, &BodyComponent)>,
) {
    let Some(SceneUpdateMessage(scene_update)) = scene_update.read().last() else {
        return;
    };
    for (mut transform, marker) in query.iter_mut() {
        let body_update = &scene_update.bodies[&marker.id];
        *transform = Transform::from_translation(Vec3::from(body_update.pos))
            .with_rotation(bevy_quat(body_update.quat));
    }
}

#[derive(Component)]
struct SceneRootMarker;

#[derive(Component)]
struct BodyComponent {
    id: usize,
}

#[derive(Component)]
pub struct TrunkComponent;

fn bevy_quat(quat: [f32; 4]) -> Quat {
    let [w, x, y, z] = quat;
    Quat::from_xyzw(x, y, z, w)
}

#[derive(Message)]
pub struct SceneDescriptionMessage(SceneDescription);

#[derive(Message)]
pub struct SceneUpdateMessage(SceneUpdate);
