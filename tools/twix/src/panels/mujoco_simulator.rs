use std::{collections::BTreeMap, fs::File, sync::Arc, thread};

use bevy::{
    asset::RenderAssetUsages,
    render::{mesh::Indices, RenderDebugFlags},
};
use bevy::{
    prelude::*,
    render::{
        camera::{ManualTextureView, ManualTextureViewHandle, ManualTextureViews, RenderTarget},
        render_resource::Texture,
        renderer::{
            RenderAdapter, RenderAdapterInfo, RenderDevice, RenderInstance, RenderQueue,
            WgpuWrapper,
        },
        settings::RenderCreation,
        RenderPlugin,
    },
};
use eframe::{
    egui::{self, load::SizedTexture, vec2, Response, Ui, Widget},
    egui_wgpu::wgpu,
    wgpu::PrimitiveTopology,
};
use futures_util::StreamExt;
use nalgebra::{Point3, Vector3};
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc;
use tokio_tungstenite::connect_async;

use crate::panel::{Panel, PanelCreationContext};

#[derive(Resource)]
struct BevyRenderTarget {
    texture: wgpu::Texture,
    texture_id: eframe::egui::TextureId,
}

struct EguiRenderPlugin {
    wgpu_state: eframe::egui_wgpu::RenderState,
}

impl Plugin for EguiRenderPlugin {
    fn build(&self, app: &mut App) {
        let instance = self.wgpu_state.instance.clone();
        let queue = self.wgpu_state.queue.clone();
        let device = self.wgpu_state.device.clone();
        let adapter = self.wgpu_state.adapter.clone();

        let size = wgpu::Extent3d {
            width: 512,
            height: 512,
            depth_or_array_layers: 1,
        };

        let texture_desc = wgpu::TextureDescriptor {
            label: Some("bevy_render_target"),
            size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            usage: wgpu::TextureUsages::TEXTURE_BINDING
                | wgpu::TextureUsages::COPY_DST
                | wgpu::TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        };

        let bevy_render_target = device.create_texture(&texture_desc);
        let texture_view = bevy_render_target.create_view(&wgpu::TextureViewDescriptor::default());

        let texture_id = self.wgpu_state.renderer.write().register_native_texture(
            &device,
            &texture_view,
            wgpu::FilterMode::Linear,
        );
        let plugin = RenderPlugin {
            render_creation: RenderCreation::manual(
                RenderDevice::new(WgpuWrapper::new(device)),
                RenderQueue(Arc::new(WgpuWrapper::new(queue))),
                RenderAdapterInfo(WgpuWrapper::new(adapter.get_info())),
                RenderAdapter(Arc::new(WgpuWrapper::new(adapter))),
                RenderInstance(Arc::new(WgpuWrapper::new(instance))),
            ),
            synchronous_pipeline_compilation: true,
            debug_flags: RenderDebugFlags::empty(),
        };

        app.add_plugins(plugin);
        app.insert_resource(BevyRenderTarget {
            texture: bevy_render_target,
            texture_id,
        });
    }
}

impl EguiRenderPlugin {
    fn new(wgpu_state: eframe::egui_wgpu::RenderState) -> Self {
        Self { wgpu_state }
    }
}

fn setup_scene(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    commands.spawn((
        PointLight {
            shadows_enabled: true,
            intensity: 10_000_000.,
            range: 100.0,
            shadow_depth_bias: 0.2,
            ..default()
        },
        Transform::from_xyz(8.0, 16.0, 8.0),
    ));
    commands.spawn((
        Mesh3d(meshes.add(Cuboid::new(0.1, 0.1, 0.7))),
        MeshMaterial3d(materials.add(Color::srgb_u8(0, 255, 0))),
        Transform::from_xyz(0.0, 0.5, 0.0),
    ));
}

fn setup_camera(
    mut commands: Commands,
    mut manual_tex_view: ResMut<ManualTextureViews>,
    texture: Res<BevyRenderTarget>,
) {
    let texture = Texture::from(texture.texture.clone());
    let manual_texture_view = ManualTextureView::with_default_format(
        texture.create_view(&wgpu::TextureViewDescriptor::default()),
        UVec2::new(512, 512),
    );
    let manual_texture_view_handle = ManualTextureViewHandle(0);
    if manual_tex_view
        .insert(manual_texture_view_handle, manual_texture_view)
        .is_some()
    {
        panic!("ManualTextureViewHandle 0 already exists");
    }

    commands.spawn((
        Camera3d::default(),
        Camera {
            target: RenderTarget::TextureView(manual_texture_view_handle),
            clear_color: Color::linear_rgba(0.3, 0.3, 0.3, 0.3).into(),
            ..Default::default()
        },
        Transform::from_xyz(1.0, 1.0, 1.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));
}

pub struct MujocoSimulatorPanel {
    rx: mpsc::Receiver<SceneUpdate>,
    bevy_app: App,
}

impl<'a> Panel<'a> for MujocoSimulatorPanel {
    const NAME: &'static str = "Mujoco Simulator";

    fn new(context: PanelCreationContext) -> Self {
        let mut bevy_app = App::new();
        bevy_app
            .add_plugins(
                DefaultPlugins
                    .build()
                    .add_before::<RenderPlugin>(EguiRenderPlugin::new(context.wgpu_state.clone()))
                    .disable::<RenderPlugin>(),
            )
            .add_systems(Startup, setup_camera)
            .add_systems(Startup, setup_scene);
        let file = File::open("/tmp/scene").unwrap();
        // let mut bytes = Vec::new();
        // file.read_to_end(&mut bytes).unwrap();
        // println!("Read bytes: {}", bytes.len());
        let scene: SceneDescription = rmp_serde::from_read(file).unwrap();
        let mut meshes = BTreeMap::new();
        for (name, mesh) in &scene.meshes {
            let mesh = bevy_app.world_mut().resource_mut::<Assets<Mesh>>().add(
                Mesh::new(
                    PrimitiveTopology::TriangleList,
                    RenderAssetUsages::RENDER_WORLD,
                )
                .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, mesh.vertices.clone())
                .with_inserted_indices(Indices::U32(mesh.faces.concat()))
                .with_computed_normals(),
            );
            meshes.insert(name.clone(), mesh);
        }
        for (name, body) in &scene.bodies {
            let mut materials = Vec::new();
            for geom in &body.geoms {
                materials.push(
                    bevy_app
                        .world_mut()
                        .resource_mut::<Assets<StandardMaterial>>()
                        .add(Color::srgba_u8(
                            (geom.rgba[0] * 255.0) as u8,
                            (geom.rgba[1] * 255.0) as u8,
                            (geom.rgba[2] * 255.0) as u8,
                            (geom.rgba[3] * 255.0) as u8,
                        )),
                );
            }
            bevy_app
                .world_mut()
                .spawn((
                    Transform::default(),
                    Visibility::default(),
                    BodyMarker { name: name.clone() },
                ))
                .with_children(|parent| {
                    for (geom, material) in body.geoms.iter().zip(materials) {
                        let Some(mesh_name) = geom.mesh.as_ref() else {
                            continue;
                        };
                        parent.spawn((
                            Transform::from_translation(geom.pos)
                                .with_rotation(bevy_quat(geom.quat)),
                            Visibility::default(),
                            Mesh3d(meshes.get(mesh_name).cloned().unwrap()),
                            MeshMaterial3d(material),
                        ));
                    }
                });
        }
        bevy_app.finish();
        bevy_app.cleanup();

        let (tx, rx) = mpsc::channel(10);

        let egui_ctx = context.egui_context.clone();
        thread::spawn(|| {
            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .unwrap();
            rt.block_on(async move {
                println!("starting background task");
                let (stream, _response) = connect_async("ws://localhost:8000/scene/subscribe")
                    .await
                    .unwrap();
                let (_sender, mut receiver) = stream.split();
                while let Some(Ok(message)) = receiver.next().await {
                    let text = message.to_text().unwrap();
                    let update: SceneUpdate = serde_json::from_str(text).unwrap();
                    tx.send(update).await.unwrap();
                    egui_ctx.request_repaint();
                }
            });
        });

        Self { rx, bevy_app }
    }
}

impl Widget for &mut MujocoSimulatorPanel {
    fn ui(self, ui: &mut Ui) -> Response {
        if let Ok(update) = self.rx.try_recv() {
            let mut query = self
                .bevy_app
                .world_mut()
                .query::<(&mut Transform, &BodyMarker)>();
            for (mut transform, marker) in query.iter_mut(self.bevy_app.world_mut()) {
                let body_update = &update.bodies[&marker.name];
                *transform = Transform::from_translation(body_update.pos)
                    .with_rotation(bevy_quat(body_update.quat));
            }
        }
        self.bevy_app.update();
        let texture_id = self
            .bevy_app
            .world()
            .get_resource::<BevyRenderTarget>()
            .unwrap()
            .texture_id;
        let image_source = egui::ImageSource::Texture(SizedTexture {
            id: texture_id,
            size: vec2(512.0, 512.0),
        });
        ui.image(image_source)
    }
}

#[derive(Component)]
struct BodyMarker {
    name: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct SceneDescription {
    meshes: BTreeMap<String, SceneMesh>,
    // textures: dict
    lights: Vec<Light>,
    bodies: BTreeMap<String, Body>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct SceneMesh {
    vertices: Vec<[f32; 3]>,
    faces: Vec<[u32; 3]>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct Light {
    name: Option<String>,
    pos: Point3<f32>,
    dir: Vector3<f32>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct Body {
    id: i64,
    parent: Option<String>,
    geoms: Vec<Geom>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct Geom {
    name: Option<String>,
    mesh: Option<String>,
    rgba: Vec<f32>,
    pos: Vec3,
    quat: Quat,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct SceneUpdate {
    time: Option<String>,
    bodies: BTreeMap<String, BodyUpdate>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
struct BodyUpdate {
    pos: Vec3,
    quat: Quat,
}

fn bevy_quat(quat: Quat) -> Quat {
    Quat::from_xyzw(quat.y, quat.z, quat.w, quat.x)
}
