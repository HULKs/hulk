use std::sync::Arc;

use bevy::render::RenderDebugFlags;
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
    egui_wgpu::{wgpu, WgpuConfiguration, WgpuSetup},
};
use eframe::{
    egui::{CentralPanel, Context},
    run_native, CreationContext, Frame, NativeOptions, Renderer,
};
use serde_json::Value;

use crate::{
    nao::Nao,
    panel::{Panel, PanelCreationContext},
};

struct EguiApp {}

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
        Mesh3d(meshes.add(Cuboid::new(1.0, 1.0, 1.0))),
        MeshMaterial3d(materials.add(Color::srgb_u8(124, 144, 255))),
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
            clear_color: Color::WHITE.into(),
            ..Default::default()
        },
        Transform::from_xyz(-2.5, 4.5, 9.0).looking_at(Vec3::ZERO, Vec3::Y),
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
        bevy_app.finish();
        bevy_app.cleanup();

        Self { bevy_app }
    }
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
