use std::sync::mpsc;

use bevy::{
    asset::RenderAssetUsages,
    camera::{RenderTarget, visibility::RenderLayers},
    prelude::*,
    render::{
        RenderApp,
        render_asset::RenderAssets,
        render_resource::{
            BufferDescriptor, BufferUsages, CommandEncoderDescriptor, Extent3d, MapMode, PollType,
            TexelCopyBufferInfo, TexelCopyBufferLayout, TextureFormat, TextureUsages,
        },
        renderer::{RenderDevice, RenderQueue},
        texture::GpuImage,
    },
    window::ExitCondition,
};
use nalgebra::{Isometry3, Matrix3, Rotation3, UnitQuaternion, Vector3};

use crate::bevy_scene::setup_field_scene;

pub(crate) const FX: f32 = 430.0;
pub(crate) const FY: f32 = FX;
pub(crate) const CX: f32 = 272.0;
pub(crate) const CY: f32 = 224.0;
pub(crate) const WIDTH: u32 = (2.0 * CX) as u32;
pub(crate) const HEIGHT: u32 = (2.0 * CY) as u32;
#[allow(dead_code)] // Used by the simulation integration built on top of this renderer.
pub(crate) const BASELINE: f32 = 0.064;

pub struct RenderedStereoRgba {
    pub left: Vec<u8>,
    pub right: Vec<u8>,
}

pub struct StereoRenderer {
    app: App,
    left_camera: Entity,
    right_camera: Entity,
    left_target: Handle<Image>,
    right_target: Handle<Image>,
}

impl StereoRenderer {
    pub fn new() -> Self {
        let mut app = App::new();
        app.add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: None,
            exit_condition: ExitCondition::DontExit,
            ..default()
        }))
        .insert_resource(GlobalAmbientLight {
            color: Color::WHITE,
            brightness: 450.0,
            ..default()
        })
        .add_systems(Startup, setup_field_scene);
        app.finish();
        app.cleanup();

        let (left, right) = {
            let mut images = app.world_mut().resource_mut::<Assets<Image>>();
            (render_target(&mut images), render_target(&mut images))
        };
        let projection = Projection::Perspective(PerspectiveProjection {
            fov: 2.0 * (HEIGHT as f32 / (2.0 * FY)).atan(),
            aspect_ratio: WIDTH as f32 / HEIGHT as f32,
            ..default()
        });
        let left_camera = app
            .world_mut()
            .spawn(sensor_camera(left.clone(), projection.clone()))
            .id();
        let right_camera = app
            .world_mut()
            .spawn(sensor_camera(right.clone(), projection))
            .id();
        for _ in 0..4 {
            app.update();
            app.world()
                .resource::<RenderDevice>()
                .wgpu_device()
                .poll(PollType::Wait {
                    submission_index: None,
                    timeout: None,
                })
                .expect("GPU polling failed");
        }

        Self {
            app,
            left_camera,
            right_camera,
            left_target: left,
            right_target: right,
        }
    }

    pub fn render(
        &mut self,
        left_camera_to_field: &Isometry3<f32>,
        right_camera_to_field: &Isometry3<f32>,
    ) -> RenderedStereoRgba {
        *self
            .app
            .world_mut()
            .entity_mut(self.left_camera)
            .get_mut::<Transform>()
            .unwrap() = sensor_transform(left_camera_to_field);
        *self
            .app
            .world_mut()
            .entity_mut(self.right_camera)
            .get_mut::<Transform>()
            .unwrap() = sensor_transform(right_camera_to_field);
        self.app.update();
        let left = readback_texture(&self.app, &self.left_target);
        let right = readback_texture(&self.app, &self.right_target);
        RenderedStereoRgba {
            left: unpad_rgba_rows(&left, WIDTH, HEIGHT),
            right: unpad_rgba_rows(&right, WIDTH, HEIGHT),
        }
    }
}

impl Default for StereoRenderer {
    fn default() -> Self {
        Self::new()
    }
}

fn render_target(images: &mut Assets<Image>) -> Handle<Image> {
    let mut image = Image::new_target_texture(WIDTH, HEIGHT, TextureFormat::Rgba8UnormSrgb, None);
    image.asset_usage = RenderAssetUsages::RENDER_WORLD;
    image.texture_descriptor.usage |= TextureUsages::COPY_SRC;
    images.add(image)
}

fn sensor_camera(target: Handle<Image>, projection: Projection) -> impl Bundle {
    (
        Camera3d::default(),
        Camera {
            clear_color: Color::srgb(0.08, 0.12, 0.16).into(),
            ..default()
        },
        projection,
        RenderTarget::Image(target.into()),
        Transform::IDENTITY,
        RenderLayers::layer(0),
    )
}

fn readback_texture(app: &App, image: &Handle<Image>) -> Vec<u8> {
    let render_world = app.sub_app(RenderApp).world();
    let gpu_images = render_world.resource::<RenderAssets<GpuImage>>();
    let gpu_image = gpu_images
        .get(image)
        .expect("render target was not prepared on the GPU");
    let render_device = render_world.resource::<RenderDevice>();
    let padded_row = RenderDevice::align_copy_bytes_per_row((WIDTH * 4) as usize);
    let buffer = render_device.create_buffer(&BufferDescriptor {
        label: Some("stereo sensor readback"),
        size: (padded_row * HEIGHT as usize) as u64,
        usage: BufferUsages::COPY_DST | BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });
    let mut encoder = render_device.create_command_encoder(&CommandEncoderDescriptor::default());
    encoder.copy_texture_to_buffer(
        gpu_image.texture.as_image_copy(),
        TexelCopyBufferInfo {
            buffer: &buffer,
            layout: TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(padded_row as u32),
                rows_per_image: None,
            },
        },
        Extent3d {
            width: WIDTH,
            height: HEIGHT,
            depth_or_array_layers: 1,
        },
    );
    render_world
        .resource::<RenderQueue>()
        .submit(std::iter::once(encoder.finish()));

    let slice = buffer.slice(..);
    let (sender, receiver) = mpsc::sync_channel(1);
    slice.map_async(MapMode::Read, move |result| {
        sender.send(result).expect("readback receiver was dropped");
    });
    render_device
        .poll(PollType::Wait {
            submission_index: None,
            timeout: None,
        })
        .expect("GPU polling failed");
    receiver
        .recv()
        .expect("GPU readback callback was dropped")
        .expect("GPU readback mapping failed");
    let data = slice
        .get_mapped_range()
        .expect("GPU readback was not mapped")
        .to_vec();
    buffer.unmap();
    data
}

fn sensor_transform(camera_to_field: &Isometry3<f32>) -> Transform {
    let field_to_bevy = Matrix3::new(1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, -1.0, 0.0);
    let optical_to_bevy = Matrix3::new(1.0, 0.0, 0.0, 0.0, -1.0, 0.0, 0.0, 0.0, -1.0);
    let rotation =
        field_to_bevy * camera_to_field.rotation.to_rotation_matrix().matrix() * optical_to_bevy;
    let quaternion =
        UnitQuaternion::from_rotation_matrix(&Rotation3::from_matrix_unchecked(rotation));
    let quaternion = quaternion.quaternion();
    Transform {
        translation: Vec3::from(bevy_position(camera_to_field.translation.vector)),
        rotation: Quat::from_xyzw(quaternion.i, quaternion.j, quaternion.k, quaternion.w),
        ..default()
    }
}

fn bevy_position(position: Vector3<f32>) -> [f32; 3] {
    [position.x, position.z, -position.y]
}

fn unpad_rgba_rows(data: &[u8], width: u32, height: u32) -> Vec<u8> {
    let tight_row = width as usize * 4;
    let padded_row = tight_row.next_multiple_of(256);
    assert_eq!(data.len(), padded_row * height as usize);
    let mut tight = Vec::with_capacity(tight_row * height as usize);
    for row in data.chunks_exact(padded_row) {
        tight.extend_from_slice(&row[..tight_row]);
    }
    tight
}

#[cfg(test)]
mod tests {
    use nalgebra::{Translation3, UnitQuaternion};

    use super::*;

    #[test]
    fn dimensions_and_intrinsics_are_centered() {
        assert_eq!((WIDTH, HEIGHT), (544, 448));
        assert_eq!((CX, CY), (WIDTH as f32 / 2.0, HEIGHT as f32 / 2.0));
        assert_eq!(FX, FY);
        assert_eq!(BASELINE, 0.064);
    }

    #[test]
    fn optical_camera_basis_converts_to_bevy() {
        let pose =
            Isometry3::from_parts(Translation3::new(1.0, 2.0, 3.0), UnitQuaternion::identity());
        let transform = sensor_transform(&pose);

        assert_eq!(transform.translation, Vec3::new(1.0, 3.0, -2.0));
        assert!((transform.rotation * Vec3::X - Vec3::X).length() < 1.0e-6);
        assert!((transform.rotation * Vec3::Y - Vec3::Z).length() < 1.0e-6);
        assert!((transform.rotation * -Vec3::Z - Vec3::Y).length() < 1.0e-6);
    }

    #[test]
    fn removes_gpu_row_padding() {
        let mut padded = vec![0xee; 256 * 2];
        padded[..12].copy_from_slice(&(0..12).collect::<Vec<_>>());
        padded[256..268].copy_from_slice(&(12..24).collect::<Vec<_>>());

        assert_eq!(unpad_rgba_rows(&padded, 3, 2), (0..24).collect::<Vec<_>>());
    }

    #[test]
    #[ignore = "requires a graphics adapter"]
    fn renders_stereo_images_on_gpu() {
        let left = crate::trajectory::Scenario::stationary()
            .sample_camera_to_field(0.0)
            .inner;
        let right = left * Isometry3::translation(BASELINE, 0.0, 0.0);
        let mut renderer = StereoRenderer::new();
        let rendered = renderer.render(&left, &right);
        let second = renderer.render(&left, &right);

        assert_eq!(rendered.left.len(), (WIDTH * HEIGHT * 4) as usize);
        assert_eq!(rendered.right.len(), rendered.left.len());
        assert_ne!(rendered.left, rendered.right);
        assert!(second.left.iter().any(|value| *value != 0));
    }
}
