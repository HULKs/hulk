use bevy::{camera::visibility::RenderLayers, light::NotShadowCaster, prelude::*};
use mujoco_rs::prelude::{MjSpec, MjtGeom, SpecItem};
use types::field_dimensions::FieldDimensions;

use super::{object::ObjectKind, visual::ObjectVisualAssets};
use crate::{
    bevy_mujoco::{MjcfObject, MujocoBody, from_mujoco},
    parameters::CurrentSimulatorParameters,
};

const GOAL_HEIGHT: f64 = 0.8;
const SUPPORT_RADIUS: f64 = 0.01;
const NET_RADIUS: f64 = 0.001;

const IDENTITY: [f64; 4] = [1.0, 0.0, 0.0, 0.0];
const ALONG_X: [f64; 4] = [
    std::f64::consts::FRAC_1_SQRT_2,
    0.0,
    std::f64::consts::FRAC_1_SQRT_2,
    0.0,
];
const ALONG_Y: [f64; 4] = [
    std::f64::consts::FRAC_1_SQRT_2,
    std::f64::consts::FRAC_1_SQRT_2,
    0.0,
    0.0,
];

#[derive(Component)]
pub struct Goal;

#[derive(Component)]
pub(super) struct GoalPartIndex(usize);

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct GoalDimensions {
    inner_width: f64,
    post_diameter: f64,
    depth: f64,
}

impl From<&FieldDimensions> for GoalDimensions {
    fn from(dimensions: &FieldDimensions) -> Self {
        Self {
            inner_width: dimensions.goal_inner_width as f64,
            post_diameter: dimensions.goal_post_diameter as f64,
            depth: dimensions.goal_depth as f64,
        }
    }
}

#[derive(Clone, Copy)]
struct GoalPrimitive {
    kind: MjtGeom,
    radius: f64,
    half_length: f64,
    position: [f64; 3],
    rotation: [f64; 4],
    collides: bool,
}

struct GoalPart {
    mesh: Handle<Mesh>,
    solid: Handle<StandardMaterial>,
    ghost: Handle<StandardMaterial>,
    transform: Transform,
}

pub struct GoalAssets {
    parts: Vec<GoalPart>,
    dimensions: GoalDimensions,
}

impl GoalAssets {
    pub fn load(world: &mut World) -> Self {
        let dimensions = GoalDimensions::from(
            &world
                .resource::<CurrentSimulatorParameters>()
                .parameters
                .field_dimensions,
        );
        let (solid, ghost) = {
            let mut materials = world.resource_mut::<Assets<StandardMaterial>>();
            (
                materials.add(StandardMaterial {
                    base_color: Color::WHITE,
                    perceptual_roughness: 0.8,
                    ..default()
                }),
                materials.add(StandardMaterial {
                    base_color: Color::srgba(1.0, 1.0, 1.0, 0.38),
                    perceptual_roughness: 0.8,
                    alpha_mode: AlphaMode::Blend,
                    ..default()
                }),
            )
        };
        let mut parts = Vec::with_capacity(70);
        for primitive in goal_primitives(dimensions) {
            let mesh = world
                .resource_mut::<Assets<Mesh>>()
                .add(mesh_from_primitive(primitive));
            parts.push(GoalPart {
                mesh,
                solid: solid.clone(),
                ghost: ghost.clone(),
                transform: from_mujoco(primitive.position, primitive.rotation),
            });
        }

        Self { parts, dimensions }
    }

    pub fn preview_center(&self) -> Vec3 {
        Vec3::new(
            -(self.dimensions.depth as f32) / 2.0,
            GOAL_HEIGHT as f32 / 2.0,
            0.0,
        )
    }

    fn mjcf_object(&self) -> MjcfObject {
        let dimensions = self.dimensions;
        MjcfObject::from_factory(move || goal_spec(dimensions), "goal")
            .with_mocap_body("goal")
            .grounded()
            .reapply_pose_on_change()
    }

    fn update_dimensions(&mut self, dimensions: GoalDimensions, meshes: &mut Assets<Mesh>) {
        let primitives = goal_primitives(dimensions);
        assert_eq!(self.parts.len(), primitives.len());
        for (part, primitive) in self.parts.iter_mut().zip(primitives) {
            meshes
                .insert(part.mesh.id(), mesh_from_primitive(primitive))
                .expect("goal mesh should exist");
            part.transform = from_mujoco(primitive.position, primitive.rotation);
        }
        self.dimensions = dimensions;
    }

    pub fn spawn_visual(
        &self,
        commands: &mut Commands,
        transform: Transform,
        ghost: bool,
        layers: RenderLayers,
    ) -> Entity {
        let root = commands
            .spawn((
                transform,
                Visibility::default(),
                layers.clone(),
                Pickable::IGNORE,
            ))
            .id();
        self.insert_visual(commands, root, ghost, layers);
        root
    }

    fn insert_visual(
        &self,
        commands: &mut Commands,
        root: Entity,
        ghost: bool,
        layers: RenderLayers,
    ) {
        commands.entity(root).with_children(|parent| {
            for (index, part) in self.parts.iter().enumerate() {
                let mut visual = parent.spawn((
                    GoalPartIndex(index),
                    Mesh3d(part.mesh.clone()),
                    MeshMaterial3d(if ghost {
                        part.ghost.clone()
                    } else {
                        part.solid.clone()
                    }),
                    part.transform,
                    layers.clone(),
                    Pickable::IGNORE,
                ));
                if ghost {
                    visual.insert(NotShadowCaster);
                }
            }
        });
    }
}

pub fn spawn(commands: &mut Commands, assets: &GoalAssets, transform: Transform) -> Entity {
    let mut goal = commands.spawn_empty();
    let entity = goal.id();
    goal.insert((
        Goal,
        ObjectKind::Goal,
        assets.mjcf_object(),
        MujocoBody::new(entity, "goal"),
        transform,
        Visibility::default(),
    ));
    assets.insert_visual(commands, entity, false, RenderLayers::layer(0));
    entity
}

pub(super) fn update_goal_dimensions(
    parameters: Res<CurrentSimulatorParameters>,
    mut assets: ResMut<ObjectVisualAssets>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut goals: Query<&mut MjcfObject, (With<Goal>, Without<GoalPartIndex>)>,
    mut visual_parts: Query<(&GoalPartIndex, &mut Transform), Without<Goal>>,
) {
    let dimensions = GoalDimensions::from(&parameters.parameters.field_dimensions);
    if assets.goal.dimensions == dimensions {
        return;
    }

    assets.goal.update_dimensions(dimensions, &mut meshes);
    for (index, mut transform) in &mut visual_parts {
        *transform = assets.goal.parts[index.0].transform;
    }
    for mut object in &mut goals {
        *object = assets.goal.mjcf_object();
    }
}

pub(crate) fn goal_spec(dimensions: GoalDimensions) -> Result<MjSpec, String> {
    let mut spec = MjSpec::new();
    let body = spec
        .world_body_mut()
        .add_body()
        .with_name("goal")
        .with_mocap(true);
    for (index, primitive) in goal_primitives(dimensions).into_iter().enumerate() {
        let geom = body.add_geom();
        geom.set_name(&format!("part_{index}"))
            .map_err(|error| error.to_string())?;
        geom.with_type(primitive.kind)
            .with_size([primitive.radius, primitive.half_length, 0.0])
            .with_pos(primitive.position)
            .with_quat(primitive.rotation)
            .with_rgba([1.0, 1.0, 1.0, 1.0]);
        if !primitive.collides {
            geom.with_contype(0).with_conaffinity(0).with_density(0.0);
        }
    }
    Ok(spec)
}

fn goal_primitives(dimensions: GoalDimensions) -> Vec<GoalPrimitive> {
    let half_span = (dimensions.inner_width + dimensions.post_diameter) / 2.0;
    let post_radius = dimensions.post_diameter / 2.0;
    let mut parts = Vec::with_capacity(70);
    let mut add = |kind, radius, half_length, position, rotation, collides| {
        parts.push(GoalPrimitive {
            kind,
            radius,
            half_length,
            position,
            rotation,
            collides,
        });
    };

    for y in [-half_span, half_span] {
        add(
            MjtGeom::mjGEOM_CYLINDER,
            SUPPORT_RADIUS,
            GOAL_HEIGHT / 2.0,
            [-dimensions.depth, y, GOAL_HEIGHT / 2.0],
            IDENTITY,
            true,
        );
    }
    for z in [SUPPORT_RADIUS, GOAL_HEIGHT] {
        add(
            MjtGeom::mjGEOM_CAPSULE,
            SUPPORT_RADIUS,
            half_span,
            [-dimensions.depth, 0.0, z],
            ALONG_Y,
            true,
        );
    }
    for y in [-half_span, half_span] {
        for z in [SUPPORT_RADIUS, GOAL_HEIGHT] {
            add(
                MjtGeom::mjGEOM_CYLINDER,
                SUPPORT_RADIUS,
                dimensions.depth / 2.0,
                [-dimensions.depth / 2.0, y, z],
                ALONG_X,
                true,
            );
        }
    }
    for y in [-half_span, half_span] {
        add(
            MjtGeom::mjGEOM_CYLINDER,
            post_radius,
            GOAL_HEIGHT / 2.0,
            [0.0, y, GOAL_HEIGHT / 2.0],
            IDENTITY,
            true,
        );
    }
    add(
        MjtGeom::mjGEOM_CAPSULE,
        post_radius,
        half_span,
        [0.0, 0.0, GOAL_HEIGHT],
        ALONG_Y,
        true,
    );

    for index in 1..=15 {
        let y = -half_span + 2.0 * half_span * index as f64 / 16.0;
        add(
            MjtGeom::mjGEOM_CYLINDER,
            NET_RADIUS,
            GOAL_HEIGHT / 2.0,
            [-dimensions.depth, y, GOAL_HEIGHT / 2.0],
            IDENTITY,
            false,
        );
    }
    for index in 2..=7 {
        let z = GOAL_HEIGHT * index as f64 / 8.0;
        add(
            MjtGeom::mjGEOM_CYLINDER,
            NET_RADIUS,
            half_span,
            [-dimensions.depth, 0.0, z],
            ALONG_Y,
            false,
        );
    }
    for y in [-half_span, half_span] {
        for index in 1..=4 {
            let x = -dimensions.depth + dimensions.depth * index as f64 / 4.0;
            add(
                MjtGeom::mjGEOM_CYLINDER,
                NET_RADIUS,
                GOAL_HEIGHT / 2.0,
                [x, y, GOAL_HEIGHT / 2.0],
                IDENTITY,
                false,
            );
        }
        for index in 2..=7 {
            let z = GOAL_HEIGHT * index as f64 / 8.0;
            add(
                MjtGeom::mjGEOM_CYLINDER,
                NET_RADIUS,
                dimensions.depth / 2.0,
                [-dimensions.depth / 2.0, y, z],
                ALONG_X,
                false,
            );
        }
    }
    for index in 1..=3 {
        let x = -dimensions.depth + dimensions.depth * index as f64 / 4.0;
        add(
            MjtGeom::mjGEOM_CYLINDER,
            NET_RADIUS,
            half_span,
            [x, 0.0, GOAL_HEIGHT],
            ALONG_Y,
            false,
        );
    }
    for index in 1..=15 {
        let y = -half_span + 2.0 * half_span * index as f64 / 16.0;
        add(
            MjtGeom::mjGEOM_CYLINDER,
            NET_RADIUS,
            dimensions.depth / 2.0,
            [-dimensions.depth / 2.0, y, GOAL_HEIGHT],
            ALONG_X,
            false,
        );
    }

    debug_assert_eq!(parts.len(), 70);
    parts
}

fn mesh_from_primitive(primitive: GoalPrimitive) -> Mesh {
    let radius = primitive.radius as f32;
    let length = 2.0 * primitive.half_length as f32;
    match primitive.kind {
        MjtGeom::mjGEOM_CAPSULE => Capsule3d::new(radius, length).mesh().build(),
        MjtGeom::mjGEOM_CYLINDER => Cylinder::new(radius, length).mesh().build(),
        kind => panic!("unsupported goal geometry {kind:?}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn goal_primitives_use_radian_rotations() {
        let mut spec = goal_spec(GoalDimensions::from(&FieldDimensions::SPL_2025)).unwrap();
        let model = spec.compile().unwrap();
        assert_eq!(model.geom_type().len(), 70);

        let axis = |geom| {
            let transform = from_mujoco(model.geom_pos()[geom], model.geom_quat()[geom]);
            (transform.rotation * Vec3::Y).abs()
        };
        assert!(axis(0).abs_diff_eq(Vec3::Y, 1e-5));
        assert!(axis(2).abs_diff_eq(Vec3::Z, 1e-5));
        assert!(axis(4).abs_diff_eq(Vec3::X, 1e-5));
    }
}
