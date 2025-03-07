use std::f32::consts::PI;

use async_runtime::AsyncRuntimePlugin;
use ball::BallPlugin;
use bevy::{pbr::DirectionalLightShadowMap, prelude::*};
use bevy_obj::ObjPlugin;
use bevy_panorbit_camera::{PanOrbitCamera, PanOrbitCameraPlugin};
use color_eyre::eyre::Result;
use field::FieldPlugin;
use fps::FpsPlugin;
use nao::NaoPlugin;
use parameters::Parameters;
use shadow_toggle::ShadowTogglePlugin;
use ui::UiPlugin;

mod async_runtime;
mod ball;
mod field;
mod fps;
mod nao;
mod parameters;
mod ring;
mod shadow_toggle;
mod ui;

fn main() -> Result<()> {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(ObjPlugin)
        .add_plugins(UiPlugin)
        .add_plugins(FpsPlugin)
        .add_plugins(PanOrbitCameraPlugin)
        .insert_resource(Parameters::default())
        .insert_resource(DirectionalLightShadowMap { size: 4096 })
        .add_plugins(BallPlugin)
        .add_plugins(FieldPlugin)
        .add_plugins(NaoPlugin)
        .add_plugins(ShadowTogglePlugin)
        .add_plugins(AsyncRuntimePlugin)
        .add_systems(Startup, setup_camera)
        .add_systems(Startup, setup_light)
        .add_systems(Startup, setup_gizmos)
        .run();
    Ok(())
}

#[derive(Component)]
struct MainCamera;

fn setup_camera(mut commands: Commands) {
    commands
        .spawn(Transform::from_translation(Vec3::ZERO).looking_at(Vec3::X, Vec3::Z))
        .with_child((
            Transform::from_translation(Vec3::new(6.0, 8.0, 0.0)).looking_at(Vec3::ZERO, Vec3::Y),
            Camera3d::default(),
            PanOrbitCamera::default(),
            MainCamera,
        ));
}

fn setup_light(mut commands: Commands) {
    commands.insert_resource(AmbientLight {
        color: Color::WHITE,
        brightness: 2000.0,
    });
    commands.spawn((
        Name::new("sun"),
        DirectionalLight {
            shadows_enabled: false,
            illuminance: 5000.0,
            ..default()
        },
        Transform {
            translation: Vec3::new(0.0, 0.0, 2.0),
            rotation: Quat::from_euler(EulerRot::XYZ, -PI / 4.0, -PI / 6.0, 0.0),
            ..default()
        },
    ));
}

fn setup_gizmos(mut gizmos: ResMut<GizmoConfigStore>) {
    gizmos.config_mut::<DefaultGizmoConfigGroup>().0.line_width = 4.0
}
