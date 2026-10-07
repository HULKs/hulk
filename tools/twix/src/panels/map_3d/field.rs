use super::{FieldDimensions, Settings, ViewerData};
use bevy::{
    asset::RenderAssetUsages,
    mesh::{Indices, PrimitiveTopology},
    prelude::*,
};
use coordinate_systems::Field;
use linear_algebra::Point2;
use std::f32::consts::{FRAC_PI_2, PI, TAU};
use types::field_dimensions::{Half, Side};

#[derive(Component)]
pub(super) struct FieldPlane;

#[derive(Component)]
pub(super) struct FieldMarkings;

pub(super) fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let field_material = materials.add(StandardMaterial {
        base_color: Color::srgb(0.04, 0.34, 0.13),
        perceptual_roughness: 0.95,
        ..default()
    });
    let markings_material = materials.add(StandardMaterial {
        base_color: Color::WHITE,
        unlit: true,
        cull_mode: None,
        ..default()
    });
    commands.spawn((
        FieldPlane,
        Mesh3d(meshes.add(field_mesh())),
        MeshMaterial3d(field_material),
        Transform::default(),
    ));
    commands.spawn((
        FieldMarkings,
        Mesh3d(meshes.add(field_markings_mesh(&FieldDimensions::SPL_2025))),
        MeshMaterial3d(markings_material),
        Transform::default(),
    ));
}
pub(super) fn update_field_plane(
    data: Res<ViewerData>,
    mut field: Single<&mut Transform, With<FieldPlane>>,
) {
    let dimensions = data
        .field_dimensions
        .as_ref()
        .map(|record| record.value)
        .unwrap_or(FieldDimensions::SPL_2025);
    let length = dimensions.length + 2.0 * dimensions.border_strip_width;
    let width = dimensions.width + 2.0 * dimensions.border_strip_width;

    field.scale = Vec3::new(length, 1.0, width);
}

pub(super) fn update_field_markings(
    data: Res<ViewerData>,
    markings: Single<&Mesh3d, With<FieldMarkings>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut rendered: Local<Option<ros_z::pubsub::PublicationId>>,
) {
    let publication = data
        .field_dimensions
        .as_ref()
        .map(|record| record.publication_id);
    if *rendered == publication {
        return;
    }
    *rendered = publication;
    let dimensions = data
        .field_dimensions
        .as_ref()
        .map(|record| record.value)
        .unwrap_or(FieldDimensions::SPL_2025);
    meshes
        .insert(markings.id(), field_markings_mesh(&dimensions))
        .expect("field markings mesh handle should be valid");
}

fn field_markings_mesh(dimensions: &FieldDimensions) -> Mesh {
    let mut mesh = FieldMarkingMesh::default();
    let line_width = dimensions.line_width.max(0.001);
    let xy = |point: Point2<Field>| [point.x(), point.y()];
    mesh.add_segment(
        xy(dimensions.t_crossing(Side::Left)),
        xy(dimensions.t_crossing(Side::Right)),
        line_width,
    );
    for side in [Side::Left, Side::Right] {
        mesh.add_segment(
            xy(dimensions.corner(Half::Own, side)),
            xy(dimensions.corner(Half::Opponent, side)),
            line_width,
        );
    }
    mesh.add_arc(
        [0.0, 0.0],
        dimensions.center_circle_diameter / 2.0,
        0.0,
        TAU,
        line_width,
    );

    for half in [Half::Own, Half::Opponent] {
        for (start, end) in [
            (
                dimensions.corner(half, Side::Left),
                dimensions.corner(half, Side::Right),
            ),
            (
                dimensions.goal_box_corner(half, Side::Left),
                dimensions.goal_box_corner(half, Side::Right),
            ),
            (
                dimensions.penalty_box_corner(half, Side::Left),
                dimensions.penalty_box_corner(half, Side::Right),
            ),
        ] {
            mesh.add_segment(xy(start), xy(end), line_width);
        }
        mesh.add_marker_cross(
            xy(dimensions.penalty_spot(half)),
            dimensions.penalty_marker_size,
            line_width,
        );
        for side in [Side::Left, Side::Right] {
            mesh.add_segment(
                xy(dimensions.goal_box_corner(half, side)),
                xy(dimensions.goal_box_goal_line_intersection(half, side)),
                line_width,
            );
            mesh.add_segment(
                xy(dimensions.penalty_box_corner(half, side)),
                xy(dimensions.penalty_box_goal_line_intersection(half, side)),
                line_width,
            );
            mesh.add_disk(
                xy(dimensions.goal_post(half, side)),
                dimensions.goal_post_diameter / 2.0,
            );
            let start = match (half, side) {
                (Half::Own, Side::Left) => -FRAC_PI_2,
                (Half::Own, Side::Right) => 0.0,
                (Half::Opponent, Side::Left) => PI,
                (Half::Opponent, Side::Right) => FRAC_PI_2,
            };
            mesh.add_arc(
                xy(dimensions.corner(half, side)),
                dimensions.corner_arc_radius,
                start,
                start + FRAC_PI_2,
                line_width,
            );
        }
    }

    mesh.finish()
}

#[derive(Default)]
struct FieldMarkingMesh {
    positions: Vec<[f32; 3]>,
    normals: Vec<[f32; 3]>,
    uvs: Vec<[f32; 2]>,
    indices: Vec<u32>,
}

impl FieldMarkingMesh {
    const HEIGHT: f32 = 0.025;

    fn add_marker_cross(&mut self, center: [f32; 2], size: f32, line_width: f32) {
        let half_size = size / 2.0;
        self.add_segment(
            [center[0] - half_size, center[1]],
            [center[0] + half_size, center[1]],
            line_width,
        );
        self.add_segment(
            [center[0], center[1] - half_size],
            [center[0], center[1] + half_size],
            line_width,
        );
    }

    fn add_segment(&mut self, start: [f32; 2], end: [f32; 2], width: f32) {
        let delta = [end[0] - start[0], end[1] - start[1]];
        let length = delta[0].hypot(delta[1]);
        if !length.is_finite() || length <= f32::EPSILON {
            return;
        }

        let half_width = width / 2.0;
        let perpendicular = [
            -delta[1] / length * half_width,
            delta[0] / length * half_width,
        ];
        self.add_quad([
            [start[0] - perpendicular[0], start[1] - perpendicular[1]],
            [end[0] - perpendicular[0], end[1] - perpendicular[1]],
            [end[0] + perpendicular[0], end[1] + perpendicular[1]],
            [start[0] + perpendicular[0], start[1] + perpendicular[1]],
        ]);
    }

    fn add_arc(&mut self, center: [f32; 2], radius: f32, start: f32, end: f32, width: f32) {
        if !radius.is_finite() || radius <= 0.0 {
            return;
        }

        let half_width = width / 2.0;
        let inner_radius = (radius - half_width).max(0.0);
        let outer_radius = radius + half_width;
        let segments = ((radius * (end - start).abs()) / 0.05).ceil() as usize;
        let segments = segments.clamp(8, 96);

        for index in 0..segments {
            let angle0 = start + (end - start) * index as f32 / segments as f32;
            let angle1 = start + (end - start) * (index + 1) as f32 / segments as f32;
            self.add_quad([
                arc_point(center, inner_radius, angle0),
                arc_point(center, inner_radius, angle1),
                arc_point(center, outer_radius, angle1),
                arc_point(center, outer_radius, angle0),
            ]);
        }
    }

    fn add_disk(&mut self, center: [f32; 2], radius: f32) {
        if !radius.is_finite() || radius <= 0.0 {
            return;
        }

        let segments = 32;
        for index in 0..segments {
            let angle0 = std::f32::consts::TAU * index as f32 / segments as f32;
            let angle1 = std::f32::consts::TAU * (index + 1) as f32 / segments as f32;
            self.add_triangle([
                center,
                arc_point(center, radius, angle1),
                arc_point(center, radius, angle0),
            ]);
        }
    }

    fn add_quad(&mut self, points: [[f32; 2]; 4]) {
        if !points.iter().flatten().all(|value| value.is_finite()) {
            return;
        }
        let base = self.positions.len() as u32;
        for point in points {
            self.add_vertex(point);
        }
        self.indices
            .extend_from_slice(&[base, base + 1, base + 2, base, base + 2, base + 3]);
    }

    fn add_triangle(&mut self, points: [[f32; 2]; 3]) {
        if !points.iter().flatten().all(|value| value.is_finite()) {
            return;
        }
        let base = self.positions.len() as u32;
        for point in points {
            self.add_vertex(point);
        }
        self.indices.extend_from_slice(&[base, base + 1, base + 2]);
    }

    fn add_vertex(&mut self, point: [f32; 2]) {
        self.positions.push([point[0], Self::HEIGHT, -point[1]]);
        self.normals.push([0.0, 1.0, 0.0]);
        self.uvs.push([0.0, 0.0]);
    }

    fn finish(self) -> Mesh {
        let mut mesh = Mesh::new(
            PrimitiveTopology::TriangleList,
            RenderAssetUsages::RENDER_WORLD,
        );
        mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, self.positions);
        mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, self.normals);
        mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, self.uvs);
        mesh.insert_indices(Indices::U32(self.indices));
        mesh
    }
}

fn arc_point(center: [f32; 2], radius: f32, angle: f32) -> [f32; 2] {
    [
        center[0] + radius * angle.cos(),
        center[1] + radius * angle.sin(),
    ]
}
fn field_mesh() -> Mesh {
    let mut mesh = Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::RENDER_WORLD,
    );
    mesh.insert_attribute(
        Mesh::ATTRIBUTE_POSITION,
        vec![
            [-0.5, 0.0, -0.5],
            [0.5, 0.0, -0.5],
            [0.5, 0.0, 0.5],
            [-0.5, 0.0, 0.5],
        ],
    );
    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, vec![[0.0, 1.0, 0.0]; 4]);
    mesh.insert_attribute(
        Mesh::ATTRIBUTE_UV_0,
        vec![[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]],
    );
    mesh.insert_indices(Indices::U32(vec![0, 2, 1, 0, 3, 2]));
    mesh
}

type FieldEntities = Or<(With<FieldPlane>, With<FieldMarkings>)>;

pub(super) fn visibility(
    settings: Res<Settings>,
    mut entities: Query<&mut Visibility, FieldEntities>,
) {
    for mut visibility in &mut entities {
        *visibility = if settings.field {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn markings_stay_finite_with_corners_and_invalid_dimensions() {
        for dimensions in [
            FieldDimensions::SPL_2025,
            FieldDimensions {
                corner_arc_radius: 0.5,
                ..FieldDimensions::SPL_2025
            },
            FieldDimensions {
                length: f32::NAN,
                corner_arc_radius: f32::INFINITY,
                ..FieldDimensions::SPL_2025
            },
        ] {
            let mesh = field_markings_mesh(&dimensions);
            let positions = mesh
                .attribute(Mesh::ATTRIBUTE_POSITION)
                .unwrap()
                .as_float3()
                .unwrap();
            assert!(!positions.is_empty());
            assert!(positions.iter().flatten().all(|value| value.is_finite()));
        }
    }
}
