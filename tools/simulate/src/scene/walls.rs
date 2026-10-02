//! Physical rebound walls at the outer edge of the field's border strip.
use bevy::prelude::*;
use mujoco_rs::prelude::{MjSpec, MjtGeom, SpecItem};
use types::field_dimensions::FieldDimensions;

use crate::{bevy_mujoco::MjcfObject, parameters::CurrentSimulatorParameters};

use super::SceneParameterUpdateSet;

const HEIGHT: f64 = 1.0;
const THICKNESS: f64 = 0.2;

struct Wall {
    center: [f64; 3],
    half_size: [f64; 3],
}

impl Wall {
    fn transform(&self) -> Transform {
        let [x, y, z] = self.center.map(|v| v as f32);
        let [hx, hy, hz] = self.half_size.map(|v| v as f32);
        Transform::from_xyz(x, z, -y).with_scale(Vec3::new(2.0 * hx, 2.0 * hz, 2.0 * hy))
    }
}

fn walls(dimensions: &FieldDimensions) -> [Wall; 4] {
    let x = f64::from(dimensions.length / 2.0 + dimensions.border_strip_width);
    let y = f64::from(dimensions.width / 2.0 + dimensions.border_strip_width);
    [
        Wall {
            center: [x + THICKNESS / 2.0, 0.0, HEIGHT / 2.0],
            half_size: [THICKNESS / 2.0, y + THICKNESS, HEIGHT / 2.0],
        },
        Wall {
            center: [-x - THICKNESS / 2.0, 0.0, HEIGHT / 2.0],
            half_size: [THICKNESS / 2.0, y + THICKNESS, HEIGHT / 2.0],
        },
        Wall {
            center: [0.0, y + THICKNESS / 2.0, HEIGHT / 2.0],
            half_size: [x, THICKNESS / 2.0, HEIGHT / 2.0],
        },
        Wall {
            center: [0.0, -y - THICKNESS / 2.0, HEIGHT / 2.0],
            half_size: [x, THICKNESS / 2.0, HEIGHT / 2.0],
        },
    ]
}

pub(crate) fn object(dimensions: FieldDimensions) -> MjcfObject {
    MjcfObject::from_factory(
        move || {
            let mut spec = MjSpec::new();
            let body = spec.world_body_mut().add_body();
            body.set_name("field_walls")
                .map_err(|error| error.to_string())?;
            for wall in walls(&dimensions) {
                body.add_geom()
                    .with_type(MjtGeom::mjGEOM_BOX)
                    .with_pos(wall.center)
                    .with_size(wall.half_size)
                    // Higher priority than the ball: use these rebound settings for contact.
                    .with_priority(2)
                    .with_condim(3)
                    .with_friction([0.1, 0.001, 0.00001])
                    // MuJoCo direct format: stiffness and small damping for near-elastic contact.
                    // https://mujoco.readthedocs.io/en/stable/modeling.html#restitution
                    .with_solref([-10000.0, -2.0])
                    .with_solimp([0.99, 0.99, 0.001, 0.5, 2.0]);
            }
            Ok(spec)
        },
        "field_walls",
    )
}

#[derive(Component)]
struct WallVisual(usize);
#[derive(Component)]
struct WallPhysics;

pub struct WallsPlugin;

impl Plugin for WallsPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, spawn)
            .add_systems(PreUpdate, update.in_set(SceneParameterUpdateSet));
    }
}

fn spawn(
    mut commands: Commands,
    parameters: Res<CurrentSimulatorParameters>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let dimensions = parameters.parameters.field_dimensions;
    commands.spawn((WallPhysics, object(dimensions)));
    let mesh = meshes.add(Cuboid::default());
    let material = materials.add(StandardMaterial {
        base_color: Color::srgba(0.15, 0.65, 1.0, 0.3),
        alpha_mode: AlphaMode::Blend,
        // Each side of the closed box has its own outward-facing surface.
        // Drawing back faces too doubles the transparent layers, whose triangles
        // are not depth-sorted within a mesh. Unlit shading keeps all four inner
        // faces equally visible regardless of the direction of the scene light.
        unlit: true,
        ..default()
    });
    let rim_material = materials.add(StandardMaterial {
        base_color: Color::srgb(0.15, 0.65, 1.0),
        unlit: true,
        ..default()
    });
    for (index, wall) in walls(&dimensions).iter().enumerate() {
        commands
            .spawn((
                WallVisual(index),
                Mesh3d(mesh.clone()),
                MeshMaterial3d(material.clone()),
                bevy::light::NotShadowCaster,
                wall.transform(),
            ))
            .with_children(|parent| {
                // An opaque cap makes both the inner and outer top edges clear,
                // even when several transparent walls overlap on screen.
                parent.spawn((
                    Mesh3d(mesh.clone()),
                    MeshMaterial3d(rim_material.clone()),
                    bevy::light::NotShadowCaster,
                    Transform::from_xyz(0.0, 0.49, 0.0).with_scale(Vec3::new(1.0, 0.02, 1.0)),
                ));
            });
    }
}

fn update(
    parameters: Res<CurrentSimulatorParameters>,
    mut physics: Single<&mut MjcfObject, With<WallPhysics>>,
    mut visuals: Query<(&WallVisual, &mut Transform)>,
) {
    if !parameters.is_changed() {
        return;
    }
    let dimensions = parameters.parameters.field_dimensions;
    **physics = object(dimensions);
    let walls = walls(&dimensions);
    for (wall, mut transform) in &mut visuals {
        *transform = walls[wall.0].transform();
    }
}
