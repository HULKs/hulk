use bevy::prelude::*;

use crate::nao::Nao;

pub struct CameraPlugin;

impl Plugin for CameraPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, setup_camera_planes);
    }
}

#[derive(Component)]
pub struct TopCameraPlane;

#[derive(Component)]
pub struct BottomCameraPlane;

fn setup_camera_planes(new_naos: Query<&Nao, Added<Nao>>) {
    println!("setup_camera_planes");
}
