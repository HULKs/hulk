use std::f32::consts::PI;

use bevy::prelude::*;

use crate::{parameters::Parameters, ring::Ring};

pub struct FieldPlugin;

impl Plugin for FieldPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, setup_field);
    }
}

fn setup_field(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    parameters: Res<Parameters>,
) {
    let field_dimensions = &parameters.field_dimensions;
    let ground_size = Vec2::new(
        field_dimensions.length + field_dimensions.border_strip_width * 2.0,
        field_dimensions.width + field_dimensions.border_strip_width * 2.0,
    );
    let line_material = materials.add(Color::srgb(1.0, 1.0, 1.0));
    let ground_line_size = Vec2::new(
        field_dimensions.line_width,
        field_dimensions.width + field_dimensions.line_width / 2.0 * 2.0,
    );
    let ground_line = meshes.add(Rectangle::from_size(ground_line_size).mesh());
    let out_line_size = Vec2::new(
        field_dimensions.length + field_dimensions.line_width / 2.0 * 2.0,
        field_dimensions.line_width,
    );
    let out_line = meshes.add(Rectangle::from_size(out_line_size).mesh());
    let center_circle = meshes.add(Ring::new(
        field_dimensions.center_circle_diameter / 2.0 - field_dimensions.line_width / 2.0,
        field_dimensions.center_circle_diameter / 2.0 + field_dimensions.line_width / 2.0,
        64,
    ));
    let goal_box_area_side_line_size = Vec2::new(
        field_dimensions.goal_box_area_length + field_dimensions.line_width / 2.0 * 2.0,
        field_dimensions.line_width,
    );
    let goal_box_area_side_line = meshes.add(Rectangle::from_size(goal_box_area_side_line_size));
    let goal_box_area_front_line_size = Vec2::new(
        field_dimensions.line_width,
        field_dimensions.goal_box_area_width + field_dimensions.line_width / 2.0 * 2.0,
    );
    let goal_box_area_front_line = meshes.add(Rectangle::from_size(goal_box_area_front_line_size));
    let penalty_area_side_line_size = Vec2::new(
        field_dimensions.penalty_area_length + field_dimensions.line_width / 2.0 * 2.0,
        field_dimensions.line_width,
    );
    let penalty_area_side_line = meshes.add(Rectangle::from_size(penalty_area_side_line_size));
    let penalty_area_front_line_size = Vec2::new(
        field_dimensions.line_width,
        field_dimensions.penalty_area_width + field_dimensions.line_width / 2.0 * 2.0,
    );
    let penalty_area_front_line = meshes.add(Rectangle::from_size(penalty_area_front_line_size));
    let penalty_marker_dash_size = Vec2::new(
        field_dimensions.penalty_marker_size,
        field_dimensions.line_width,
    );
    let penalty_marker_dash = meshes.add(Rectangle::from_size(penalty_marker_dash_size));
    let goal_post_material = materials.add(Color::srgb(1.0, 1.0, 1.0));
    const GOAL_POST_HEIGHT: f32 = 0.8;
    let goal_post = meshes.add(
        Cylinder::new(field_dimensions.goal_post_diameter / 2.0, GOAL_POST_HEIGHT)
            .mesh()
            .resolution(32)
            .segments(1),
    );
    let goal_crossbar = meshes.add(
        Cylinder::new(
            field_dimensions.goal_post_diameter / 2.0,
            field_dimensions.goal_inner_width + field_dimensions.goal_post_diameter * 2.0,
        )
        .mesh()
        .resolution(32)
        .segments(1),
    );
    const GOAL_SUPPORT_STRUCTURE_THICKNESS: f32 = 0.03;
    let goal_support_structure_x_length = field_dimensions.goal_depth
        - field_dimensions.line_width / 2.0
        + GOAL_SUPPORT_STRUCTURE_THICKNESS / 2.0
        - field_dimensions.goal_post_diameter / 2.0;
    let goal_support_structure_x = meshes.add(Cuboid::new(
        goal_support_structure_x_length,
        GOAL_SUPPORT_STRUCTURE_THICKNESS,
        GOAL_SUPPORT_STRUCTURE_THICKNESS,
    ));
    let goal_support_structure_y_length = field_dimensions.goal_inner_width
        + field_dimensions.goal_post_diameter / 2.0 * 2.0
        + GOAL_SUPPORT_STRUCTURE_THICKNESS / 2.0 * 2.0;
    let goal_support_structure_y = meshes.add(Cuboid::new(
        GOAL_SUPPORT_STRUCTURE_THICKNESS,
        goal_support_structure_y_length,
        GOAL_SUPPORT_STRUCTURE_THICKNESS,
    ));
    let goal_support_structure_z_length = GOAL_POST_HEIGHT;
    let goal_support_structure_z = meshes.add(Cuboid::new(
        GOAL_SUPPORT_STRUCTURE_THICKNESS,
        GOAL_SUPPORT_STRUCTURE_THICKNESS,
        goal_support_structure_z_length,
    ));

    let mut material: StandardMaterial = Color::srgb(0.3, 0.5, 0.3).into();
    material.perceptual_roughness = 1.0;
    commands.spawn((
        Name::new("field"),
        Mesh3d(meshes.add(Rectangle::from_size(ground_size).mesh())),
        MeshMaterial3d(materials.add(material)),
    ));

    commands.spawn((
        Name::new("center_line"),
        Mesh3d(ground_line.clone()),
        MeshMaterial3d(line_material.clone()),
        Transform::from_translation(Vec3::new(0.0, 0.0, 0.001)),
    ));
    commands.spawn((
        Name::new("center_circle"),
        Mesh3d(center_circle.clone()),
        MeshMaterial3d(line_material.clone()),
        Transform::from_translation(Vec3::new(0.0, 0.0, 0.001)),
    ));
    commands.spawn((
        Name::new("kick_off_mark"),
        Mesh3d(penalty_marker_dash.clone()),
        MeshMaterial3d(line_material.clone()),
        Transform::from_translation(Vec3::new(0.0, 0.0, 0.001)),
    ));

    for rotation in [0.0, PI] {
        let rotation = Quat::from_rotation_z(rotation);
        commands.spawn((
            Name::new("ground_line"),
            Mesh3d(ground_line.clone()),
            MeshMaterial3d(line_material.clone()),
            Transform::from_translation(
                rotation * Vec3::new(-field_dimensions.length / 2.0, 0.0, 0.001),
            ),
        ));
        commands.spawn((
            Name::new("out_line"),
            Mesh3d(out_line.clone()),
            MeshMaterial3d(line_material.clone()),
            Transform::from_translation(
                rotation * Vec3::new(0.0, -field_dimensions.width / 2.0, 0.001),
            ),
        ));
        commands.spawn((
            Name::new("goal_box_area_side_line"),
            Mesh3d(goal_box_area_side_line.clone()),
            MeshMaterial3d(line_material.clone()),
            Transform::from_translation(
                rotation
                    * Vec3::new(
                        -field_dimensions.length / 2.0
                            + field_dimensions.goal_box_area_length / 2.0,
                        -field_dimensions.goal_box_area_width / 2.0,
                        0.001,
                    ),
            ),
        ));
        commands.spawn((
            Name::new("goal_box_area_side_line"),
            Mesh3d(goal_box_area_side_line.clone()),
            MeshMaterial3d(line_material.clone()),
            Transform::from_translation(
                rotation
                    * Vec3::new(
                        -field_dimensions.length / 2.0
                            + field_dimensions.goal_box_area_length / 2.0,
                        field_dimensions.goal_box_area_width / 2.0,
                        0.001,
                    ),
            ),
        ));
        commands.spawn((
            Name::new("goal_box_area_front_line"),
            Mesh3d(goal_box_area_front_line.clone()),
            MeshMaterial3d(line_material.clone()),
            Transform::from_translation(
                rotation
                    * Vec3::new(
                        -field_dimensions.length / 2.0 + field_dimensions.goal_box_area_length,
                        0.0,
                        0.001,
                    ),
            ),
        ));
        commands.spawn((
            Name::new("penalty_area_side_line"),
            Mesh3d(penalty_area_side_line.clone()),
            MeshMaterial3d(line_material.clone()),
            Transform::from_translation(
                rotation
                    * Vec3::new(
                        -field_dimensions.length / 2.0 + field_dimensions.penalty_area_length / 2.0,
                        -field_dimensions.penalty_area_width / 2.0,
                        0.001,
                    ),
            ),
        ));
        commands.spawn((
            Name::new("penalty_area_side_line"),
            Mesh3d(penalty_area_side_line.clone()),
            MeshMaterial3d(line_material.clone()),
            Transform::from_translation(
                rotation
                    * Vec3::new(
                        -field_dimensions.length / 2.0 + field_dimensions.penalty_area_length / 2.0,
                        field_dimensions.penalty_area_width / 2.0,
                        0.001,
                    ),
            ),
        ));
        commands.spawn((
            Name::new("penalty_area_front_line"),
            Mesh3d(penalty_area_front_line.clone()),
            MeshMaterial3d(line_material.clone()),
            Transform::from_translation(
                rotation
                    * Vec3::new(
                        -field_dimensions.length / 2.0 + field_dimensions.penalty_area_length,
                        0.0,
                        0.001,
                    ),
            ),
        ));
        commands.spawn((
            Name::new("penalty_marker"),
            Mesh3d(penalty_marker_dash.clone()),
            MeshMaterial3d(line_material.clone()),
            Transform::from_translation(
                rotation
                    * Vec3::new(
                        -field_dimensions.length / 2.0
                            + field_dimensions.penalty_marker_distance
                            + field_dimensions.penalty_marker_size / 2.0,
                        0.0,
                        0.001,
                    ),
            ),
        ));
        commands.spawn((
            Name::new("penalty_marker"),
            Mesh3d(penalty_marker_dash.clone()),
            MeshMaterial3d(line_material.clone()),
            Transform::from_translation(
                rotation
                    * Vec3::new(
                        -field_dimensions.length / 2.0
                            + field_dimensions.penalty_marker_distance
                            + field_dimensions.penalty_marker_size / 2.0,
                        0.0,
                        0.001,
                    ),
            )
            .with_rotation(Quat::from_rotation_z(PI / 2.0)),
        ));
        let goal_post_center = Vec2::new(
            -field_dimensions.length / 2.0 - field_dimensions.goal_post_diameter / 2.0
                + field_dimensions.line_width / 2.0,
            -field_dimensions.goal_inner_width / 2.0 - field_dimensions.goal_post_diameter / 2.0,
        );
        commands.spawn((
            Name::new("goal_post"),
            Mesh3d(goal_post.clone()),
            MeshMaterial3d(goal_post_material.clone()),
            Transform::from_translation(
                rotation
                    * Vec3::new(
                        goal_post_center.x,
                        goal_post_center.y,
                        GOAL_POST_HEIGHT / 2.0,
                    ),
            )
            .with_rotation(Quat::from_rotation_x(PI / 2.0)),
        ));
        commands.spawn((
            Name::new("goal_post"),
            Mesh3d(goal_post.clone()),
            MeshMaterial3d(goal_post_material.clone()),
            Transform::from_translation(
                rotation
                    * Vec3::new(
                        goal_post_center.x,
                        -goal_post_center.y,
                        GOAL_POST_HEIGHT / 2.0,
                    ),
            )
            .with_rotation(Quat::from_rotation_x(PI / 2.0)),
        ));
        commands.spawn((
            Name::new("goal_crossbar"),
            Mesh3d(goal_crossbar.clone()),
            MeshMaterial3d(goal_post_material.clone()),
            Transform::from_translation(
                rotation * Vec3::new(goal_post_center.x, 0.0, GOAL_POST_HEIGHT),
            ),
        ));
        commands.spawn((
            Name::new("goal_support_structure"),
            Mesh3d(goal_support_structure_x.clone()),
            MeshMaterial3d(goal_post_material.clone()),
            Transform::from_translation(
                rotation
                    * Vec3::new(
                        -field_dimensions.length / 2.0 + field_dimensions.line_width / 2.0
                            - goal_support_structure_x_length / 2.0
                            - field_dimensions.goal_post_diameter / 2.0,
                        goal_post_center.y,
                        GOAL_SUPPORT_STRUCTURE_THICKNESS / 2.0,
                    ),
            ),
        ));
        commands.spawn((
            Name::new("goal_support_structure"),
            Mesh3d(goal_support_structure_x.clone()),
            MeshMaterial3d(goal_post_material.clone()),
            Transform::from_translation(
                rotation
                    * Vec3::new(
                        -field_dimensions.length / 2.0 + field_dimensions.line_width / 2.0
                            - goal_support_structure_x_length / 2.0
                            - field_dimensions.goal_post_diameter / 2.0,
                        -goal_post_center.y,
                        GOAL_SUPPORT_STRUCTURE_THICKNESS / 2.0,
                    ),
            ),
        ));
        commands.spawn((
            Name::new("goal_support_structure"),
            Mesh3d(goal_support_structure_x.clone()),
            MeshMaterial3d(goal_post_material.clone()),
            Transform::from_translation(
                rotation
                    * Vec3::new(
                        -field_dimensions.length / 2.0 + field_dimensions.line_width / 2.0
                            - goal_support_structure_x_length / 2.0
                            - field_dimensions.goal_post_diameter / 2.0,
                        goal_post_center.y,
                        GOAL_POST_HEIGHT - GOAL_SUPPORT_STRUCTURE_THICKNESS / 2.0,
                    ),
            ),
        ));
        commands.spawn((
            Name::new("goal_support_structure"),
            Mesh3d(goal_support_structure_x.clone()),
            MeshMaterial3d(goal_post_material.clone()),
            Transform::from_translation(
                rotation
                    * Vec3::new(
                        -field_dimensions.length / 2.0 + field_dimensions.line_width / 2.0
                            - goal_support_structure_x_length / 2.0
                            - field_dimensions.goal_post_diameter / 2.0,
                        -goal_post_center.y,
                        GOAL_POST_HEIGHT - GOAL_SUPPORT_STRUCTURE_THICKNESS / 2.0,
                    ),
            ),
        ));
        commands.spawn((
            Name::new("goal_support_structure"),
            Mesh3d(goal_support_structure_y.clone()),
            MeshMaterial3d(goal_post_material.clone()),
            Transform::from_translation(
                rotation
                    * Vec3::new(
                        -field_dimensions.length / 2.0 + field_dimensions.line_width / 2.0
                            - field_dimensions.goal_depth,
                        0.0,
                        GOAL_SUPPORT_STRUCTURE_THICKNESS / 2.0,
                    ),
            ),
        ));
        commands.spawn((
            Name::new("goal_support_structure"),
            Mesh3d(goal_support_structure_y.clone()),
            MeshMaterial3d(goal_post_material.clone()),
            Transform::from_translation(
                rotation
                    * Vec3::new(
                        -field_dimensions.length / 2.0 + field_dimensions.line_width / 2.0
                            - field_dimensions.goal_depth,
                        0.0,
                        GOAL_POST_HEIGHT - GOAL_SUPPORT_STRUCTURE_THICKNESS / 2.0,
                    ),
            ),
        ));
        commands.spawn((
            Name::new("goal_support_structure"),
            Mesh3d(goal_support_structure_z.clone()),
            MeshMaterial3d(goal_post_material.clone()),
            Transform::from_translation(
                rotation
                    * Vec3::new(
                        -field_dimensions.length / 2.0 + field_dimensions.line_width / 2.0
                            - field_dimensions.goal_depth,
                        goal_post_center.y,
                        GOAL_POST_HEIGHT / 2.0,
                    ),
            ),
        ));
        commands.spawn((
            Name::new("goal_support_structure"),
            Mesh3d(goal_support_structure_z.clone()),
            MeshMaterial3d(goal_post_material.clone()),
            Transform::from_translation(
                rotation
                    * Vec3::new(
                        -field_dimensions.length / 2.0 + field_dimensions.line_width / 2.0
                            - field_dimensions.goal_depth,
                        -goal_post_center.y,
                        GOAL_POST_HEIGHT / 2.0,
                    ),
            ),
        ));
    }
}
