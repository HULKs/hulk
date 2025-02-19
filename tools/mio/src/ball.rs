use bevy::prelude::*;

use crate::parameters::Parameters;

pub struct BallPlugin;

impl Plugin for BallPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, setup_ball);
    }
}

#[derive(Resource)]
pub struct BallAssets {
    pub mesh: Handle<Mesh>,
    pub material: Handle<StandardMaterial>,
}

fn setup_ball(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    parameters: Res<Parameters>,
) {
    let field_dimensions = &parameters.field_dimensions;
    let base = asset_server.load("textures/ball_base.png");
    let normal = asset_server.load("textures/ball_normal.png");
    let mesh = meshes.add(Sphere::new(field_dimensions.ball_radius).mesh().uv(30, 30));
    let material = materials.add(StandardMaterial {
        base_color: Color::WHITE,
        perceptual_roughness: 0.8,
        base_color_texture: Some(base),
        normal_map_texture: Some(normal),
        ..default()
    });
    commands.insert_resource(BallAssets { mesh, material });
}
