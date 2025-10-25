use std::{collections::BTreeMap, fs::File, sync::Arc};

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
    wgpu::{Extent3d, PrimitiveTopology, TextureDimension, TextureFormat},
};
use nalgebra::{Point3, Vector3};
use serde::{Deserialize, Serialize};

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
    // commands.spawn((
    //     Mesh3d(meshes.add(Cuboid::new(1.0, 1.0, 1.0))),
    //     MeshMaterial3d(materials.add(Color::srgb_u8(124, 144, 255))),
    //     Transform::from_xyz(0.0, 0.5, 0.0),
    // ));
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
                .with_inserted_indices(Indices::U32(mesh.faces.concat())),
            );
            meshes.insert(name.clone(), mesh);
        }
        for (name, body) in &scene.bodies {
            let mut materials = Vec::new();
            for geom in &body.geoms {
                let texture = bevy_app
                    .world_mut()
                    .resource_mut::<Assets<Image>>()
                    .add(uv_debug_texture());
                materials.push(
                    bevy_app
                        .world_mut()
                        .resource_mut::<Assets<StandardMaterial>>()
                        .add(StandardMaterial {
                            base_color_texture: Some(texture),
                            ..default()
                        }), // .add(Color::linear_rgba(
                            //     geom.rgba[0],
                            //     geom.rgba[1],
                            //     geom.rgba[2],
                            //     geom.rgba[3],
                            // )),
                );
            }
            bevy_app
                .world_mut()
                .spawn((Transform::default(), Visibility::default()))
                .with_children(|parent| {
                    for (geom, material) in body.geoms.iter().zip(materials) {
                        let Some(mesh_name) = geom.mesh.as_ref() else {
                            continue;
                        };
                        parent.spawn((
                            Transform::from_translation(geom.pos).with_rotation(geom.quat),
                            Visibility::default(),
                            Mesh3d(meshes.get(mesh_name).cloned().unwrap()),
                            MeshMaterial3d(material),
                        ));
                    }
                });
        }
        // dbg!(scene);
        bevy_app.finish();
        bevy_app.cleanup();

        Self { bevy_app }
    }
}

/// Creates a colorful test pattern
fn uv_debug_texture() -> Image {
    const TEXTURE_SIZE: usize = 8;

    let mut palette: [u8; 32] = [
        255, 102, 159, 255, 255, 159, 102, 255, 236, 255, 102, 255, 121, 255, 102, 255, 102, 255,
        198, 255, 102, 198, 255, 255, 121, 102, 255, 255, 236, 102, 255, 255,
    ];

    let mut texture_data = [0; TEXTURE_SIZE * TEXTURE_SIZE * 4];
    for y in 0..TEXTURE_SIZE {
        let offset = TEXTURE_SIZE * y * 4;
        texture_data[offset..(offset + TEXTURE_SIZE * 4)].copy_from_slice(&palette);
        palette.rotate_right(4);
    }

    Image::new_fill(
        Extent3d {
            width: TEXTURE_SIZE as u32,
            height: TEXTURE_SIZE as u32,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        &texture_data,
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::RENDER_WORLD,
    )
}

impl Widget for &mut MujocoSimulatorPanel {
    fn ui(self, ui: &mut Ui) -> Response {
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
