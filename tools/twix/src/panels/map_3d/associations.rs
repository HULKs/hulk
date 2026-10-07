use super::{
    CameraMatrix, Field, FieldMarkAssociations, Isometry3, PoseSource, Robot, Settings, ViewerData,
    empty_mesh,
};
use bevy::{asset::RenderAssetUsages, mesh::PrimitiveTopology, prelude::*};

use super::{Observation, ObservationContext};
use projection::Projection;
use ros_z_debug::SampleRecord;
use std::sync::Arc;
use types::time_wrapper::TimeWrapper;

pub(super) struct Observations {
    frames: Observation<TimeWrapper<FieldMarkAssociations>>,
}

impl Observations {
    pub(super) fn new(context: &impl ObservationContext) -> color_eyre::Result<Self> {
        Ok(Self {
            frames: Observation::new(
                context,
                "field_mark_association/visual_localization_local",
                64,
                Default::default(),
            )?,
        })
    }

    pub(super) fn current_frames(
        &self,
        namespace: &str,
        frame_id: Option<(u64, u64)>,
    ) -> Vec<Arc<SampleRecord<TimeWrapper<FieldMarkAssociations>>>> {
        self.frames
            .all(namespace)
            .into_iter()
            .filter(|frame| {
                Some((frame.value.inner.epoch, frame.value.inner.generation)) == frame_id
            })
            .collect()
    }
}

#[derive(Component)]
pub(super) struct FieldMarkAssociationLines;

pub(super) fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let association_material = materials.add(StandardMaterial {
        base_color: Color::srgb(1.0, 0.1, 0.72),
        unlit: true,
        ..default()
    });
    commands.spawn((
        FieldMarkAssociationLines,
        Mesh3d(meshes.add(empty_mesh(PrimitiveTopology::LineList))),
        MeshMaterial3d(association_material),
        Transform::default(),
        Visibility::Hidden,
    ));
}
pub(super) fn update(
    data: Res<ViewerData>,
    settings: Res<Settings>,
    mut lines: Single<(&Mesh3d, &mut Visibility), With<FieldMarkAssociationLines>>,
    mut meshes: ResMut<Assets<Mesh>>,
) {
    let (Some(associations), Some(camera_matrix), Some(field_to_robot)) = (
        data.field_mark_associations.as_ref(),
        data.camera_matrix.as_ref(),
        data.localization,
    ) else {
        *lines.1 = Visibility::Hidden;
        return;
    };

    if !settings.associations
        || settings.pose_source != PoseSource::Localization
        || associations.value.inner.associations.is_empty()
    {
        *lines.1 = Visibility::Hidden;
        return;
    }

    meshes
        .insert(
            lines.0.id(),
            field_mark_associations_mesh(&associations.value.inner, camera_matrix, field_to_robot),
        )
        .expect("field mark association mesh handle should be valid");
    *lines.1 = Visibility::Visible;
}
fn field_mark_associations_mesh(
    associations: &FieldMarkAssociations,
    camera_matrix: &CameraMatrix,
    field_to_robot: Isometry3<Field, Robot>,
) -> Mesh {
    let robot_to_field = field_to_robot.inverse();
    let ground_to_field = robot_to_field * camera_matrix.ground_to_robot;
    let mut positions = Vec::with_capacity(associations.associations.len() * 10);

    for association in &associations.associations {
        let Some(back_projected) = camera_matrix
            .pixel_to_ground(association.detection)
            .ok()
            .map(|ground| ground_to_field * ground.extend(0.0))
        else {
            continue;
        };
        let field_point = association.field_point;
        positions.push(field_point_position(back_projected));
        positions.push(field_point_position(field_point));
        add_field_cross(&mut positions, back_projected, 0.07);
        add_field_cross(&mut positions, field_point, 0.1);
    }

    let mut mesh = Mesh::new(PrimitiveTopology::LineList, RenderAssetUsages::RENDER_WORLD);
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
    mesh
}

fn add_field_cross(positions: &mut Vec<[f32; 3]>, point: linear_algebra::Point3<Field>, size: f32) {
    let x = point.x();
    let y = point.y();
    let z = point.z();
    positions.push(field_position(x - size, y, z));
    positions.push(field_position(x + size, y, z));
    positions.push(field_position(x, y - size, z));
    positions.push(field_position(x, y + size, z));
}

fn field_point_position(point: linear_algebra::Point3<Field>) -> [f32; 3] {
    field_position(point.x(), point.y(), point.z())
}

fn field_position(x: f32, y: f32, z: f32) -> [f32; 3] {
    [x, z + 0.08, -y]
}
