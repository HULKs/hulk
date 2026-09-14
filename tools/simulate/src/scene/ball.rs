use std::f32::consts::FRAC_PI_2;

use bevy::{
    camera::visibility::RenderLayers, image::ImageLoaderSettings, light::NotShadowCaster,
    prelude::*,
};
use mujoco_rs::prelude::{MjSpec, MjtGeom, MjtJoint, SpecItem};

use super::{object::ObjectKind, visual::ObjectVisualAssets};
use crate::{
    bevy_mujoco::{MjcfObject, MujocoBody},
    parameters::{BallParameters, CurrentSimulatorParameters},
};

const BALL_BASE_COLOR: &str = "textures/football_base_color.png";
const BALL_NORMAL_MAP: &str = "textures/football_normal.png";

#[derive(Component)]
pub struct Ball;

pub struct BallAssets {
    mesh: Handle<Mesh>,
    solid: Handle<StandardMaterial>,
    ghost: Handle<StandardMaterial>,
    radius: f32,
    parameters: BallParameters,
}

impl BallAssets {
    pub fn load(world: &mut World) -> Self {
        let current = world.resource::<CurrentSimulatorParameters>();
        let radius = current.parameters.field_dimensions.ball_radius;
        let parameters = current.parameters.ball.clone();
        let (base_color_texture, normal_map_texture) = {
            let asset_server = world.resource::<AssetServer>();
            (
                asset_server.load(BALL_BASE_COLOR),
                asset_server
                    .load_builder()
                    .with_settings(|settings: &mut ImageLoaderSettings| settings.is_srgb = false)
                    .load(BALL_NORMAL_MAP),
            )
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

pub fn spawn(commands: &mut Commands, assets: &BallAssets, transform: Transform) -> Entity {
    let mut ball = commands.spawn_empty();
    let entity = ball.id();
    ball.insert((
        Ball,
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
