use std::f32::consts::FRAC_PI_2;

use bevy::{
    camera::visibility::RenderLayers,
    image::{CompressedImageFormats, ImageLoaderSettings, ImageType},
    light::NotShadowCaster,
    prelude::*,
    render::render_resource::TextureFormat,
};
use mujoco_rs::prelude::{MjSpec, MjtGeom, MjtJoint, SpecItem};

use super::{object::ObjectKind, visual::ObjectVisualAssets};
use crate::{
    bevy_mujoco::{MjcfObject, MujocoBody, MujocoWorld},
    parameters::{BallParameters, CurrentSimulatorParameters},
};

const BALL_NORMAL_MAP: &str = "textures/football_normal.png";

#[derive(Component)]
pub struct Ball;

/// Live balls in insertion order, independent of entity index reuse.
#[derive(Default, Resource)]
pub struct SpawnedBalls(pub Vec<Entity>);

pub fn first_position(world: &MujocoWorld, balls: &SpawnedBalls) -> color_eyre::Result<[f64; 3]> {
    Ok(first_pose(world, balls)?.0)
}

/// MuJoCo world position and orientation (quaternion in w, x, y, z order).
pub fn first_pose(
    world: &MujocoWorld,
    balls: &SpawnedBalls,
) -> color_eyre::Result<([f64; 3], [f64; 4])> {
    let ball = balls.0.first().ok_or_else(|| {
        color_eyre::eyre::eyre!("No ball in the scene. Drag a ball onto the field first.")
    })?;
    let data = world.data();
    let ball = data
        .body(&format!("object_{}_ball", ball.to_bits()))
        .ok_or_else(|| color_eyre::eyre::eyre!("The first ball is not ready in MuJoCo yet."))?;
    let view = ball.view(data);
    let position = view.xpos;
    let rotation = view.xquat;
    Ok((
        [position[0], position[1], position[2]],
        [rotation[0], rotation[1], rotation[2], rotation[3]],
    ))
}

/// Linear velocity of the first ball in MuJoCo's world frame (m/s).
pub fn first_velocity(world: &MujocoWorld, balls: &SpawnedBalls) -> color_eyre::Result<[f64; 3]> {
    let ball = balls.0.first().ok_or_else(|| {
        color_eyre::eyre::eyre!("No ball in the scene. Drag a ball onto the field first.")
    })?;
    let data = world.data();
    let joint = data
        .joint(&format!("object_{}_ball_free_joint", ball.to_bits()))
        .ok_or_else(|| color_eyre::eyre::eyre!("The first ball is not ready in MuJoCo yet."))?;
    // The translational DOFs of a free joint are expressed in world coordinates.
    let velocity = joint.view(data).qvel;
    Ok([velocity[0], velocity[1], velocity[2]])
}

pub fn record_spawn(event: On<Add<Ball>>, mut balls: ResMut<SpawnedBalls>) {
    balls.0.push(event.entity);
}

pub fn record_removal(event: On<Remove<Ball>>, mut balls: ResMut<SpawnedBalls>) {
    balls.0.retain(|entity| *entity != event.entity);
}

pub struct BallAssets {
    mesh: Handle<Mesh>,
    solid: Handle<StandardMaterial>,
    selected: Handle<StandardMaterial>,
    ghost: Handle<StandardMaterial>,
    radius: f32,
    parameters: BallParameters,
}

impl BallAssets {
    pub fn load(world: &mut World) -> Self {
        let current = world.resource::<CurrentSimulatorParameters>();
        let radius = current.parameters.field_dimensions.ball_radius;
        let parameters = current.parameters.ball.clone();
        let base_color_texture = world
            .resource_mut::<Assets<Image>>()
            .add(ball_color_texture());
        let normal_map_texture = {
            let asset_server = world.resource::<AssetServer>();
            asset_server
                .load_builder()
                .with_settings(|settings: &mut ImageLoaderSettings| settings.is_srgb = false)
                .load(BALL_NORMAL_MAP)
        };
        let mesh = world.resource_mut::<Assets<Mesh>>().add(ball_mesh(radius));
        let mut materials = world.resource_mut::<Assets<StandardMaterial>>();
        Self {
            mesh,
            solid: materials.add(StandardMaterial {
                base_color_texture: Some(base_color_texture.clone()),
                normal_map_texture: Some(normal_map_texture.clone()),
                perceptual_roughness: 0.8,
                ..default()
            }),
            selected: materials.add(StandardMaterial {
                base_color: Color::srgb(1.0, 0.75, 0.2),
                base_color_texture: Some(base_color_texture.clone()),
                normal_map_texture: Some(normal_map_texture.clone()),
                emissive: LinearRgba::new(0.35, 0.18, 0.0, 1.0),
                perceptual_roughness: 0.8,
                ..default()
            }),
            ghost: materials.add(StandardMaterial {
                base_color: Color::srgba(1.0, 1.0, 1.0, 0.38),
                base_color_texture: Some(base_color_texture),
                normal_map_texture: Some(normal_map_texture),
                perceptual_roughness: 0.8,
                alpha_mode: AlphaMode::Blend,
                ..default()
            }),
            radius,
            parameters,
        }
    }

    pub fn radius(&self) -> f32 {
        self.radius
    }

    pub(super) fn material(&self, selected: bool) -> Handle<StandardMaterial> {
        if selected {
            self.selected.clone()
        } else {
            self.solid.clone()
        }
    }

    fn mjcf_object(&self) -> MjcfObject {
        let radius = self.radius as f64;
        let parameters = self.parameters.clone();
        MjcfObject::from_factory(move || ball_spec(radius, &parameters), "ball")
            .with_free_joint("ball_free_joint")
            .grounded()
    }

    fn set_parameters(
        &mut self,
        radius: f32,
        parameters: BallParameters,
        meshes: &mut Assets<Mesh>,
    ) {
        if self.radius.to_bits() != radius.to_bits() {
            meshes
                .insert(self.mesh.id(), ball_mesh(radius))
                .expect("ball mesh should exist");
        }
        self.radius = radius;
        self.parameters = parameters;
    }

    pub fn spawn_visual(
        &self,
        commands: &mut Commands,
        transform: Transform,
        ghost: bool,
        layers: RenderLayers,
    ) -> Entity {
        let mut visual = commands.spawn((
            Mesh3d(self.mesh.clone()),
            MeshMaterial3d(if ghost {
                self.ghost.clone()
            } else {
                self.solid.clone()
            }),
            transform,
            layers,
            Pickable::IGNORE,
        ));
        if ghost {
            visual.insert(NotShadowCaster);
        }
        visual.id()
    }
}

fn ball_color_texture() -> Image {
    // PNG grayscale is loaded as R8: the PBR shader then samples (r, 0, 0),
    // turning white panels red and preventing blue material tints from working.
    // Expand the embedded grayscale artwork into actual RGB color channels.
    Image::from_buffer(
        include_bytes!("../../assets/textures/football_base_color.png"),
        ImageType::Extension("png"),
        CompressedImageFormats::NONE,
        true,
        default(),
        default(),
    )
    .expect("embedded football PNG is valid")
    .convert(TextureFormat::Rgba8UnormSrgb)
    .expect("football texture supports RGBA conversion")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn soccer_texture_has_neutral_rgb_panels_for_material_tinting() {
        let image = ball_color_texture();
        assert_eq!(
            image.texture_descriptor.format,
            TextureFormat::Rgba8UnormSrgb
        );
        let pixels = image.data.as_ref().unwrap();
        assert!(
            pixels
                .chunks_exact(4)
                .all(|p| p[0] == p[1] && p[1] == p[2] && p[3] == 255)
        );
        assert!(pixels.chunks_exact(4).any(|p| p[0] == 255));
        assert!(pixels.chunks_exact(4).any(|p| p[0] == 0));
    }
}

pub fn spawn(commands: &mut Commands, assets: &BallAssets, transform: Transform) -> Entity {
    let mut ball = commands.spawn_empty();
    let entity = ball.id();
    ball.insert((
        Ball,
        Pickable::default(),
        ObjectKind::Ball,
        assets.mjcf_object(),
        MujocoBody::new(entity, "ball"),
        Mesh3d(assets.mesh.clone()),
        MeshMaterial3d(assets.solid.clone()),
        transform,
    ));
    entity
}

pub fn update_ball_dimensions(
    parameters: Res<CurrentSimulatorParameters>,
    mut assets: ResMut<ObjectVisualAssets>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut balls: Query<&mut MjcfObject, With<Ball>>,
) {
    let radius = parameters.parameters.field_dimensions.ball_radius;
    let ball_parameters = &parameters.parameters.ball;
    if assets.ball.radius.to_bits() == radius.to_bits()
        && assets.ball.parameters == *ball_parameters
    {
        return;
    }

    assets
        .ball
        .set_parameters(radius, ball_parameters.clone(), &mut meshes);
    for mut object in &mut balls {
        *object = assets.ball.mjcf_object();
    }
}

pub(crate) fn ball_spec(radius: f64, parameters: &BallParameters) -> Result<MjSpec, String> {
    let mut spec = MjSpec::new();
    let body = spec.world_body_mut().add_body();
    body.set_name("ball").map_err(|error| error.to_string())?;

    let joint = body.add_joint();
    joint
        .set_name("ball_free_joint")
        .map_err(|error| error.to_string())?;
    joint
        .with_type(MjtJoint::mjJNT_FREE)
        .with_damping([parameters.joint_damping as f64, 0.0, 0.0])
        .with_frictionloss(parameters.joint_friction_loss as f64);

    let geom = body.add_geom();
    geom.set_name("ball").map_err(|error| error.to_string())?;
    geom.with_type(MjtGeom::mjGEOM_SPHERE)
        .with_size([radius, 0.0, 0.0])
        .with_mass(parameters.mass as f64)
        .with_friction(parameters.friction.map(f64::from))
        .with_solref(parameters.solref.map(f64::from))
        .with_solimp(parameters.solimp.map(f64::from))
        .with_priority(1)
        .with_condim(6);

    Ok(spec)
}

fn ball_mesh(radius: f32) -> Mesh {
    let mut mesh = Sphere::new(radius)
        .mesh()
        .uv(64, 32)
        .transformed_by(Transform::from_rotation(Quat::from_rotation_x(-FRAC_PI_2)));
    mesh.generate_tangents()
        .expect("UV sphere should support tangent generation");
    mesh
}
