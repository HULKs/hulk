use std::{collections::BTreeMap, f32::consts::FRAC_PI_2, sync::Arc, thread, time::Duration};

use bevy::{
    asset::RenderAssetUsages,
    ecs::relationship::RelatedSpawnerCommands,
    input::{
        mouse::{MouseButtonInput, MouseMotion, MouseScrollUnit, MouseWheel},
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
        self, load::SizedTexture, Event, Image, ImageSource, MouseWheelUnit, PointerButton, Pos2,
        Response, Sense, Ui, Widget,
    },
    egui_wgpu::wgpu,
    wgpu::PrimitiveTopology,
};
use futures_util::{SinkExt, StreamExt};
use log::{debug, info};
use nalgebra::Isometry3;
use simulation_message::{
    ConnectionInfo, Geom, GeomVariant, SceneDescription, SceneUpdate, ServerMessageKind,
    SimulatorMessage,
};
use tokio::{select, sync::mpsc};
use tokio_tungstenite::{connect_async, tungstenite::Message};
use types::robot_kinematics::RobotKinematics;

use crate::{
    panel::{Panel, PanelCreationContext},
    value_buffer::BufferHandle,
};

#[derive(Resource)]
struct BevyRenderTarget {
    texture: wgpu::Texture,
    texture_id: egui::TextureId,
    output_size: egui::Vec2,
    wgpu_state: eframe::egui_wgpu::RenderState,
}

impl BevyRenderTarget {
    const TEXTURE_HANDLE: ManualTextureViewHandle = ManualTextureViewHandle(0);

    fn set_output_size(&mut self, size: egui::Vec2) {
        if self.texture.size().width < size.x as u32 || self.texture.size().height < size.y as u32 {
            let mut new_size = Vec2::new(
                self.texture.size().width as f32,
                self.texture.size().height as f32,
            );
            while new_size.x < size.x || new_size.y < size.y {
                new_size *= 2.0;
            }
            debug!("New render texture size: {new_size}");
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

fn setup_scene(mut commands: Commands) {
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
}

fn setup_camera(
    mut commands: Commands,
    mut texture_views: ResMut<ManualTextureViews>,
    render_target: Res<BevyRenderTarget>,
) {
    let texture = Texture::from(render_target.texture.clone());
    if texture_views
        .insert(
            BevyRenderTarget::TEXTURE_HANDLE,
            ManualTextureView::with_default_format(
                texture.create_view(&wgpu::TextureViewDescriptor::default()),
                UVec2::new(512, 512),
            ),
        )
        .is_some()
    {
        panic!("ManualTextureViewHandle 0 already exists");
    }

    commands.spawn((
        Camera3d::default(),
        Camera {
            target: RenderTarget::TextureView(BevyRenderTarget::TEXTURE_HANDLE),
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
    mut manual_texture_view: ResMut<ManualTextureViews>,
) {
    let texture = Texture::from(target.texture.clone());
    manual_texture_view.insert(
        BevyRenderTarget::TEXTURE_HANDLE,
        ManualTextureView::with_default_format(
            texture.create_view(&wgpu::TextureViewDescriptor::default()),
            UVec2::new(target.texture.size().width, target.texture.size().width),
        ),
    );
    camera.viewport = Some(Viewport {
        physical_size: UVec2::new(target.output_size.x as u32, target.output_size.y as u32),
        ..Viewport::default()
    });
}

fn update_active_camera(
    camera: Single<(Entity, &mut Camera)>,
    target: Res<BevyRenderTarget>,
    mut active_camera: ResMut<ActiveCameraData>,
) {
    active_camera.entity = Some(camera.0);
    active_camera.viewport_size = Some(Vec2::new(target.output_size.x, target.output_size.y));
    active_camera.window_size = Some(Vec2::new(target.output_size.x, target.output_size.y));
    active_camera.manual = true;
}

pub struct MujocoSimulatorPanel {
    update_receiver: mpsc::Receiver<ServerMessageKind>,
    bevy_app: App,

    kinematics: BufferHandle<RobotKinematics>,
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
            .init_resource::<KinematicsResource>()
            .add_plugins(PanOrbitCameraPlugin)
            .init_gizmo_group::<DefaultGizmoConfigGroup>()
            .add_event::<SceneDescriptionEvent>()
            .add_event::<SceneUpdateEvent>()
            .add_systems(Startup, setup_camera)
            .add_systems(Startup, setup_scene)
            .add_systems(Update, draw_gizmos)
            .add_systems(Update, update_bodies)
            .add_systems(Update, spawn_mujoco_scene)
            .add_systems(Update, update_active_camera);
        bevy_app.finish();
        bevy_app.cleanup();

        let (update_sender, update_receiver) = mpsc::channel(10);

        let egui_ctx = context.egui_context.clone();
        thread::spawn(|| {
            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .expect("failed to build async runtime");
            rt.block_on(async move {
                loop {
                    let Ok((stream, _response)) = connect_async("ws://localhost:8000/")
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
                                let Some(Ok(message)) = maybe_message else { println!("websocket receive failed"); break; };
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

        let kinematics = context
            .nao
            .subscribe_value("Control.main_outputs.robot_kinematics");
        Self {
            update_receiver,
            bevy_app,
            kinematics,
        }
    }
}

impl Widget for &mut MujocoSimulatorPanel {
    fn ui(self, ui: &mut Ui) -> Response {
        ui.ctx().request_repaint();
        self.process_scene_updates();
        let response = ui.allocate_response(ui.available_size(), Sense::all());
        self.process_egui_input(ui, &response);

        let mut render_target = self.bevy_app.world_mut().resource_mut::<BevyRenderTarget>();
        render_target.set_output_size(response.rect.size() * ui.pixels_per_point());
        let image = Image::new(render_target.image_source())
            .maintain_aspect_ratio(false)
            .fit_to_exact_size(response.rect.size())
            .uv(render_target.uv());

        if let Ok(Some(kinematics)) = self.kinematics.get_last_value() {
            self.bevy_app
                .world_mut()
                .insert_resource(KinematicsResource { value: kinematics });
        };
        self.bevy_app.update();

        ui.put(response.rect, image)
    }
}

#[derive(Resource, Default)]
struct KinematicsResource {
    value: RobotKinematics,
}

fn draw_gizmos(
    robot: Single<(&GlobalTransform, &TrunkComponent)>,
    kinematics: Res<KinematicsResource>,
    mut gizmos: Gizmos,
) {
    let mut draw = |pose: Isometry3<f32>| {
        let (translation, rotation) =
            (kinematics.value.torso.torso_to_robot.inner.inverse() * pose).into();
        gizmos.axes(
            *robot.0 * Transform::from_isometry(Isometry3d::new(translation, rotation)),
            0.1,
        );
    };
    draw(kinematics.value.head.neck_to_robot.inner);
    draw(kinematics.value.head.head_to_robot.inner);
    draw(kinematics.value.left_arm.inner_shoulder_to_robot.inner);
    draw(kinematics.value.left_arm.outer_shoulder_to_robot.inner);
    draw(kinematics.value.left_arm.upper_arm_to_robot.inner);
    draw(kinematics.value.left_arm.forearm_to_robot.inner);
    draw(kinematics.value.right_arm.inner_shoulder_to_robot.inner);
    draw(kinematics.value.right_arm.outer_shoulder_to_robot.inner);
    draw(kinematics.value.right_arm.upper_arm_to_robot.inner);
    draw(kinematics.value.right_arm.forearm_to_robot.inner);
    draw(kinematics.value.left_leg.pelvis_to_robot.inner);
    draw(kinematics.value.left_leg.hip_to_robot.inner);
    draw(kinematics.value.left_leg.thigh_to_robot.inner);
    draw(kinematics.value.left_leg.tibia_to_robot.inner);
    draw(kinematics.value.left_leg.ankle_to_robot.inner);
    draw(kinematics.value.left_leg.foot_to_robot.inner);
    draw(kinematics.value.right_leg.pelvis_to_robot.inner);
    draw(kinematics.value.right_leg.hip_to_robot.inner);
    draw(kinematics.value.right_leg.thigh_to_robot.inner);
    draw(kinematics.value.right_leg.tibia_to_robot.inner);
    draw(kinematics.value.right_leg.ankle_to_robot.inner);
    draw(kinematics.value.right_leg.foot_to_robot.inner);
}

fn update_bodies(
    mut scene_update: EventReader<SceneUpdateEvent>,
    mut query: Query<(&mut Transform, &BodyComponent)>,
) {
    let Some(SceneUpdateEvent(scene_update)) = scene_update.read().last() else {
        return;
    };
    for (mut transform, marker) in query.iter_mut() {
        let body_update = &scene_update.bodies[&marker.id];
        *transform = Transform::from_translation(Vec3::from(body_update.pos))
            .with_rotation(bevy_quat(body_update.quat));
    }
}

fn spawn_geom(
    entity_commands: &mut RelatedSpawnerCommands<'_, ChildOf>,
    materials: &mut Assets<StandardMaterial>,
    meshes: &mut Assets<Mesh>,
    mesh_handles: &BTreeMap<usize, Handle<Mesh>>,
    geom: &Geom,
) {
    let material = materials.add(Color::srgba_u8(
        (geom.rgba[0] * 255.0) as u8,
        (geom.rgba[1] * 255.0) as u8,
        (geom.rgba[2] * 255.0) as u8,
        (geom.rgba[3] * 255.0) as u8,
    ));

    match &geom.geom_variant {
        GeomVariant::Mesh { mesh_index } => entity_commands.spawn((
            Transform::from_translation(Vec3::from(geom.pos)).with_rotation(bevy_quat(geom.quat)),
            Visibility::default(),
            Mesh3d(mesh_handles[mesh_index].clone()),
            MeshMaterial3d(material),
        )),
        GeomVariant::Sphere { radius } => entity_commands.spawn((
            Transform::from_translation(Vec3::from(geom.pos)).with_rotation(bevy_quat(geom.quat)),
            Visibility::default(),
            Mesh3d(meshes.add(Sphere::new(*radius))),
            MeshMaterial3d(material),
        )),
        GeomVariant::Box {
            extent: [hx, hy, hz],
        } => entity_commands.spawn((
            Transform::from_translation(Vec3::from(geom.pos)).with_rotation(bevy_quat(geom.quat)),
            Visibility::default(),
            Mesh3d(meshes.add(Cuboid::new(*hx, *hy, *hz))),
            MeshMaterial3d(material),
        )),
        GeomVariant::Plane {
            normal: [nx, ny, nz],
        } => entity_commands.spawn((
            Transform::from_translation(Vec3::from(geom.pos)).with_rotation(bevy_quat(geom.quat)),
            Visibility::default(),
            Mesh3d(meshes.add(Plane3d::new(Vec3::new(*nx, *ny, *nz), Vec2::splat(100.0)))),
            MeshMaterial3d(material),
        )),
        GeomVariant::Cylinder {
            radius,
            half_height,
        } => entity_commands.spawn((
            Transform::from_translation(Vec3::from(geom.pos)).with_rotation(bevy_quat(geom.quat)),
            Visibility::default(),
            Mesh3d(meshes.add(Cylinder::new(*radius, *half_height))),
            MeshMaterial3d(material),
        )),
    };
}

fn spawn_mujoco_scene(
    mut scene_description: EventReader<SceneDescriptionEvent>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut commands: Commands,
) {
    let Some(SceneDescriptionEvent(scene)) = scene_description.read().last() else {
        return;
    };
    // TODO(oleflb): Add MuJoCo marker component used to cleanup before spawning

    let mesh_handles: BTreeMap<_, _> = scene
        .meshes
        .iter()
        .map(|(id, mesh)| {
            let handle = meshes.add(
                Mesh::new(
                    PrimitiveTopology::TriangleList,
                    RenderAssetUsages::RENDER_WORLD,
                )
                .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, mesh.vertices.clone())
                .with_inserted_indices(Indices::U32(mesh.faces.concat()))
                .with_computed_normals(),
            );
            (*id, handle)
        })
        .collect();

    let scene_root = commands
        .spawn((
            SceneRootMarker,
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
                spawn_geom(parent, &mut materials, &mut meshes, &mesh_handles, geom);
            }
        });
        if body.parent.is_none() && body.name.as_ref().is_some_and(|name| name == "Trunk") {
            parent.insert(TrunkComponent);
        }
    }
}

impl MujocoSimulatorPanel {
    fn process_scene_updates(&mut self) {
        while let Ok(update) = self.update_receiver.try_recv() {
            match update {
                ServerMessageKind::SceneUpdate(scene_update) => {
                    self.bevy_app
                        .world_mut()
                        .send_event(SceneUpdateEvent(scene_update));
                }
                ServerMessageKind::SceneDescription(scene_description) => {
                    self.bevy_app
                        .world_mut()
                        .send_event(SceneDescriptionEvent(scene_description));
                }
                _ => info!("Received unexpected simulator data"),
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
                    // TODO: Forward these events
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
                        let mut mouse = world.resource_mut::<Events<MouseMotion>>();
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
                        let mut buttons = world.resource_mut::<Events<MouseButtonInput>>();
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
                    Event::MouseWheel {
                        unit,
                        delta,
                        modifiers: _,
                    } => {
                        let unit = match unit {
                            MouseWheelUnit::Point => MouseScrollUnit::Pixel,
                            MouseWheelUnit::Line => MouseScrollUnit::Line,
                            MouseWheelUnit::Page => {
                                unimplemented!("this seems to be unused anyways")
                            }
                        };
                        let mut buttons = world.resource_mut::<Events<MouseWheel>>();
                        buttons.send(MouseWheel {
                            unit,
                            x: delta.x,
                            y: delta.y,
                            window: Entity::PLACEHOLDER,
                        });
                    }
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
    id: usize,
}

#[derive(Component)]
struct TrunkComponent;

fn bevy_quat(quat: [f32; 4]) -> Quat {
    let [w, x, y, z] = quat;
    Quat::from_xyzw(x, y, z, w)
}

#[derive(Event)]
pub struct SceneDescriptionEvent(SceneDescription);

#[derive(Event)]
pub struct SceneUpdateEvent(SceneUpdate);
