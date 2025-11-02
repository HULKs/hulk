use std::{
    collections::BTreeMap, f32::consts::FRAC_PI_2, fs::File, sync::Arc, thread, time::Duration,
};

use bevy::{
    asset::RenderAssetUsages,
    input::{
        mouse::{MouseButtonInput, MouseMotion},
        ButtonState,
    },
    render::{camera::Viewport, mesh::Indices, RenderDebugFlags},
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
use bevy_panorbit_camera::{ActiveCameraData, PanOrbitCamera, PanOrbitCameraPlugin};
use eframe::{
    egui::{
        self, load::SizedTexture, Event, Image, ImageSource, PointerButton, Pos2, Response, Sense,
        Ui, Widget,
    },
    egui_wgpu::wgpu,
    wgpu::PrimitiveTopology,
};
use futures_util::{SinkExt, StreamExt};
use nalgebra::{Point3, Vector3};
use serde::{Deserialize, Serialize};
use simulation_message::ConnectionInfo;
use tokio::{select, sync::mpsc};
use tokio_tungstenite::{connect_async, tungstenite::Message};

use crate::panel::{Panel, PanelCreationContext};

#[derive(Resource)]
struct BevyRenderTarget {
    texture: wgpu::Texture,
    texture_id: egui::TextureId,
    output_size: egui::Vec2,
    wgpu_state: eframe::egui_wgpu::RenderState,
}

impl BevyRenderTarget {
    fn set_output_size(&mut self, size: egui::Vec2) {
        if self.texture.size().width < size.x as u32 || self.texture.size().height < size.y as u32 {
            let mut new_size = Vec2::new(
                self.texture.size().width as f32,
                self.texture.size().height as f32,
            );
            while new_size.x < size.x || new_size.y < size.y {
                new_size *= 2.0;
            }
            println!("New render texture size: {}", new_size);
            (self.texture, self.texture_id) = Self::create_texture(new_size, &self.wgpu_state);
        }
        self.output_size = size;
    }

    fn uv(&self) -> egui::Rect {
        egui::Rect::from_min_max(
            Pos2::ZERO,
            (self.output_size / self.allocated_size()).to_pos2(),
        )
    }

    fn image_source<'a>(&self) -> ImageSource<'a> {
        ImageSource::Texture(SizedTexture {
            id: self.texture_id,
            size: self.allocated_size(),
        })
    }

    fn allocated_size(&self) -> egui::Vec2 {
        egui::Vec2::new(
            self.texture.size().width as f32,
            self.texture.size().height as f32,
        )
    }

    fn create_texture(
        size: Vec2,
        wgpu_state: &eframe::egui_wgpu::RenderState,
    ) -> (wgpu::Texture, egui::TextureId) {
        let size = wgpu::Extent3d {
            width: size.x as u32,
            height: size.y as u32,
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

        let bevy_render_target = wgpu_state.device.create_texture(&texture_desc);
        let texture_view = bevy_render_target.create_view(&wgpu::TextureViewDescriptor::default());

        let texture_id = wgpu_state.renderer.write().register_native_texture(
            &wgpu_state.device,
            &texture_view,
            wgpu::FilterMode::Linear,
        );
        (bevy_render_target, texture_id)
    }
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

        let (bevy_render_target, texture_id) =
            BevyRenderTarget::create_texture(Vec2::new(512.0, 512.0), &self.wgpu_state);
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
            output_size: egui::Vec2::new(512.0, 512.0),
            wgpu_state: self.wgpu_state.clone(),
        });
        app.add_systems(PostUpdate, update_camera_render_target);
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
        Mesh3d(meshes.add(Plane3d::new(Vec3::Y, Vec2::new(5.0, 5.0)))),
        MeshMaterial3d(materials.add(Color::srgb_u8(0, 255, 0))),
        Transform::from_xyz(0.0, 0.0, 0.0),
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
            viewport: Some(Viewport {
                physical_size: UVec2::new(1, 1),
                ..Viewport::default()
            }),
            clear_color: Color::linear_rgba(0.3, 0.3, 0.3, 0.3).into(),
            ..Default::default()
        },
        Transform::from_xyz(1.0, 1.0, 1.0).looking_at(Vec3::ZERO, Vec3::Y),
        PanOrbitCamera::default(),
    ));
}

fn update_camera_render_target(
    mut camera: Single<&mut Camera>,
    target: Res<BevyRenderTarget>,
    mut manual_tex_view: ResMut<ManualTextureViews>,
) {
    let texture = Texture::from(target.texture.clone());
    let manual_texture_view = ManualTextureView::with_default_format(
        texture.create_view(&wgpu::TextureViewDescriptor::default()),
        UVec2::new(target.texture.size().width, target.texture.size().width),
    );
    let manual_texture_view_handle = ManualTextureViewHandle(0);
    manual_tex_view.insert(manual_texture_view_handle, manual_texture_view);
    camera.target = RenderTarget::TextureView(manual_texture_view_handle);
    camera.viewport = Some(Viewport {
        physical_size: UVec2::new(target.output_size.x as u32, target.output_size.y as u32),
        ..Viewport::default()
    });
}

fn update_active_camera(
    camera: Single<(Entity, &mut Camera)>,
    target: Res<BevyRenderTarget>,
    mut active_cam: ResMut<ActiveCameraData>,
) {
    active_cam.entity = Some(camera.0);
    active_cam.viewport_size = Some(Vec2::new(target.output_size.x, target.output_size.y));
    active_cam.window_size = Some(Vec2::new(target.output_size.x, target.output_size.y));
    active_cam.manual = true;
}

pub struct MujocoSimulatorPanel {
    update_receiver: mpsc::Receiver<SceneMessage>,
    // command_sender: mpsc::Sender<ServerCommand>,
    bevy_app: App,
}

// #[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
// pub enum ServerCommand {
//     Reset,
// }

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
            .add_plugins(PanOrbitCameraPlugin)
            .add_systems(Startup, setup_camera)
            .add_systems(Startup, setup_scene)
            .add_systems(Update, update_active_camera);
        let file = File::open("/tmp/scene").unwrap();
        spawn_scene(&mut bevy_app, rmp_serde::from_read(file).unwrap());
        bevy_app.finish();
        bevy_app.cleanup();

        let (update_sender, update_receiver) = mpsc::channel(10);
        // let (command_sender, mut command_receiver) = mpsc::channel(10);

        let egui_ctx = context.egui_context.clone();
        thread::spawn(|| {
            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .unwrap();
            rt.block_on(async move {
                loop {
                    let Ok((stream, _response)) = connect_async("ws://localhost:8000/scene/subscribe")
                        .await else {
                            println!("Websocket connection failed, retrying...");
                            tokio::time::sleep(Duration::from_secs(1)).await;
                            continue;
                        };
                            println!("Websocket connected");
                    let (mut sender, mut receiver) = stream.split();
                    let initial_request = ConnectionInfo::viewer();
                    sender.send(Message::text(serde_json::to_string(&initial_request).unwrap())).await.unwrap();
                    loop {
                        select! {
                            maybe_message = receiver.next() => {
                                let Some(Ok(message)) = maybe_message else { println!("websocket receive failed"); break; };
                                let message: SceneMessage = match message {
                                    Message::Binary(bytes) => SceneMessage::Description(rmp_serde::from_slice(&bytes).unwrap()),
                                    Message::Text(text) => SceneMessage::Update(serde_json::from_str(text.as_str()).unwrap()),
                                    _ => continue
                                };
                                update_sender.send(message).await.unwrap();
                                egui_ctx.request_repaint();
                            }
                            // maybe_command = command_receiver.recv() => {
                            //     let Some(command) = maybe_command else { println!("command receive failed"); return; };
                            //     sender.send(tungstenite::Message::text(dbg!(serde_json::to_string(&command).unwrap()))).await.unwrap();
                            // }
                        }
                    }
                }
            });
        });

        Self {
            update_receiver,
            bevy_app,
            // command_sender,
        }
    }
}

fn spawn_scene(bevy_app: &mut App, scene: SceneDescription) {
    if let Some(mut query) = bevy_app
        .world()
        .try_query_filtered::<Entity, With<SceneRootMarker>>()
    {
        if let Ok(previous_scene) = query.single(bevy_app.world()) {
            bevy_app.world_mut().despawn(previous_scene);
        }
    }

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
    let scene_root = bevy_app
        .world_mut()
        .spawn((
            SceneRootMarker,
            Transform::from_rotation(Quat::from_rotation_x(-FRAC_PI_2)),
        ))
        .id();
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
                BodyComponent { name: name.clone() },
            ))
            .set_parent_in_place(scene_root)
            .with_children(|parent| {
                for (geom, material) in body.geoms.iter().zip(materials) {
                    let Some(mesh_name) = geom.mesh.as_ref() else {
                        continue;
                    };
                    parent.spawn((
                        Transform::from_translation(geom.pos).with_rotation(bevy_quat(geom.quat)),
                        Visibility::default(),
                        Mesh3d(meshes.get(mesh_name).cloned().unwrap()),
                        MeshMaterial3d(material),
                    ));
                }
            });
    }
}

impl Widget for &mut MujocoSimulatorPanel {
    fn ui(self, ui: &mut Ui) -> Response {
        ui.ctx().request_repaint();
        self.process_scene_updates();
        let response = ui.allocate_response(ui.available_size(), Sense::all());
        self.process_egui_input(ui, &response);

        let mut render_target = self
            .bevy_app
            .world_mut()
            .get_resource_mut::<BevyRenderTarget>()
            .unwrap();
        render_target.set_output_size(response.rect.size() * ui.pixels_per_point());
        let image = Image::new(render_target.image_source())
            .maintain_aspect_ratio(false)
            .fit_to_exact_size(response.rect.size())
            .uv(render_target.uv());

        self.bevy_app.update();

        ui.put(response.rect, image)
    }
}

impl MujocoSimulatorPanel {
    fn process_scene_updates(&mut self) {
        while let Ok(update) = self.update_receiver.try_recv() {
            match update {
                SceneMessage::Description(scene_description) => {
                    spawn_scene(&mut self.bevy_app, scene_description);
                }
                SceneMessage::Update(scene_update) => {
                    let mut query = self
                        .bevy_app
                        .world_mut()
                        .query::<(&mut Transform, &BodyComponent)>();
                    for (mut transform, marker) in query.iter_mut(self.bevy_app.world_mut()) {
                        let body_update = &scene_update.bodies[&marker.name];
                        *transform = Transform::from_translation(body_update.pos)
                            .with_rotation(bevy_quat(body_update.quat));
                    }
                }
            }
        }
    }

    fn process_egui_input(&mut self, ui: &mut Ui, response: &Response) {
        if !response.hovered() && !response.is_pointer_button_down_on() && !response.drag_stopped()
        {
            return;
        };

        let world = self.bevy_app.world_mut();
        ui.input(|input| {
            for event in &input.events {
                match event {
                    // Event::Copy => todo!(),
                    // Event::Cut => todo!(),
                    // Event::Paste(_) => todo!(),
                    // Event::Text(_) => todo!(),
                    // Event::Key {
                    //     key,
                    //     physical_key,
                    //     pressed,
                    //     repeat,
                    //     modifiers,
                    // } => {}
                    // Event::PointerMoved(pos2) => {}
                    Event::MouseMoved(egui::Vec2 { x, y }) => {
                        let mut mouse = world.get_resource_mut::<Events<MouseMotion>>().unwrap();
                        mouse.send(MouseMotion {
                            delta: Vec2 { x: *x, y: *y },
                        });
                    }
                    Event::PointerButton {
                        pos: _,
                        button,
                        pressed,
                        modifiers: _,
                    } => {
                        let button = match button {
                            PointerButton::Primary => MouseButton::Left,
                            PointerButton::Secondary => MouseButton::Right,
                            PointerButton::Middle => MouseButton::Middle,
                            PointerButton::Extra1 => MouseButton::Forward,
                            PointerButton::Extra2 => MouseButton::Back,
                        };
                        let mut buttons = world
                            .get_resource_mut::<Events<MouseButtonInput>>()
                            .unwrap();
                        buttons.send(MouseButtonInput {
                            button,
                            state: if *pressed {
                                ButtonState::Pressed
                            } else {
                                ButtonState::Released
                            },
                            window: Entity::PLACEHOLDER,
                        });
                    }
                    // Event::PointerGone => {}
                    // Event::Zoom(_) => todo!(),
                    // Event::Ime(ime_event) => todo!(),
                    // Event::Touch {
                    //     device_id,
                    //     id,
                    //     phase,
                    //     pos,
                    //     force,
                    // } => todo!(),
                    // Event::MouseWheel {
                    //     unit,
                    //     delta,
                    //     modifiers,
                    // } => todo!(),
                    _ => {}
                }
            }
        });
    }
}

#[derive(Component)]
struct SceneRootMarker;

#[derive(Component)]
struct BodyComponent {
    name: String,
}

enum SceneMessage {
    Description(SceneDescription),
    Update(SceneUpdate),
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
