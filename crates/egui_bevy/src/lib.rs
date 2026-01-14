use std::sync::Arc;

use bevy::{
    input::{
        mouse::{MouseButtonInput, MouseMotion, MouseScrollUnit, MouseWheel},
        ButtonState,
    },
    prelude::*,
    render::{
        camera::{
            ManualTextureView, ManualTextureViewHandle, ManualTextureViews, RenderTarget, Viewport,
        },
        render_resource::Texture,
        renderer::{
            RenderAdapter, RenderAdapterInfo, RenderDevice, RenderInstance, RenderQueue,
            WgpuWrapper,
        },
        settings::RenderCreation,
        view::RenderLayers,
        RenderDebugFlags, RenderPlugin,
    },
};
use bevy_panorbit_camera::{ActiveCameraData, PanOrbitCamera, PanOrbitCameraPlugin};
use eframe::{
    egui::{
        self, load::SizedTexture, Event, ImageSource, MouseWheelUnit, PointerButton, Pos2,
        Response, Sense, Ui, Widget,
    },
    wgpu,
};
use log::debug;

pub struct BevyWidget {
    pub bevy_app: App,
}

impl BevyWidget {
    pub fn new(wgpu_state: eframe::egui_wgpu::RenderState) -> Self {
        let mut bevy_app = App::new();

        bevy_app.add_plugins(
            DefaultPlugins
                .build()
                .add_before::<RenderPlugin>(EguiRenderPlugin::new(wgpu_state))
                .disable::<RenderPlugin>(),
        );

        Self { bevy_app }
    }
}

impl Widget for &mut BevyWidget {
    fn ui(self, ui: &mut eframe::egui::Ui) -> eframe::egui::Response {
        let response = ui.allocate_response(ui.available_size(), Sense::all());
        process_egui_input(self.bevy_app.world_mut(), ui, &response);

        let mut render_target = self.bevy_app.world_mut().resource_mut::<BevyRenderTarget>();
        render_target.set_output_size(response.rect.size() * ui.pixels_per_point());
        let image = egui::Image::new(render_target.image_source())
            .maintain_aspect_ratio(false)
            .fit_to_exact_size(response.rect.size())
            .uv(render_target.uv());

        self.bevy_app.update();

        ui.put(response.rect, image)
    }
}

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

        app.add_plugins(plugin)
            .insert_resource(BevyRenderTarget {
                texture: bevy_render_target,
                texture_id,
                output_size: egui::Vec2::new(512.0, 512.0),
                wgpu_state: self.wgpu_state.clone(),
            })
            .add_plugins(PanOrbitCameraPlugin)
            .add_systems(Startup, setup_camera)
            .add_systems(Startup, setup_scene)
            .add_systems(Update, update_active_camera)
            .add_systems(Update, update_camera_render_target);
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
                UVec2::new(
                    render_target.output_size.x as u32,
                    render_target.output_size.y as u32,
                ),
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
        RenderLayers::layer(0),
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

fn process_egui_input(world: &mut World, ui: &mut Ui, response: &Response) {
    if !response.hovered() && !response.is_pointer_button_down_on() && !response.drag_stopped() {
        return;
    };

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
