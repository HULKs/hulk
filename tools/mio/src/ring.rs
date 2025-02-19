use std::f32::consts::TAU;

use bevy::{
    prelude::*,
    render::{mesh::Indices, render_asset::RenderAssetUsages, render_resource::PrimitiveTopology},
};

pub struct Ring {
    inner_radius: f32,
    outer_radius: f32,
    sides: usize,
}

impl Ring {
    pub fn new(inner_radius: f32, outer_radius: f32, sides: usize) -> Self {
        Self {
            inner_radius,
            outer_radius,
            sides,
        }
    }
}

impl From<Ring> for Mesh {
    fn from(value: Ring) -> Self {
        let sides = value.sides;
        let inner_radius = value.inner_radius;
        let outer_radius = value.outer_radius;

        let mut positions = Vec::with_capacity(sides);
        let mut normals = Vec::with_capacity(sides);
        let mut uvs = Vec::with_capacity(sides);

        let step = TAU / (sides - 1) as f32;
        for i in 0..sides {
            let theta = std::f32::consts::FRAC_PI_2 - i as f32 * step;
            let (sin, cos) = theta.sin_cos();

            positions.push([cos * outer_radius, sin * outer_radius, 0.0]);
            normals.push([0.0, 0.0, 1.0]);
            uvs.push([0.5 * (cos + 1.0), 1.0 - 0.5 * (sin + 1.0)]);

            positions.push([cos * inner_radius, sin * inner_radius, 0.0]);
            normals.push([0.0, 0.0, 1.0]);
            uvs.push([0.5 * (cos + 1.0), 1.0 - 0.5 * (sin + 1.0)]);
        }

        let indices = (0..sides as u32 * 2).collect();
        let mut mesh = Mesh::new(
            PrimitiveTopology::TriangleStrip,
            RenderAssetUsages::default(),
        );
        mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
        mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
        mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uvs);
        mesh.insert_indices(Indices::U32(indices));
        mesh
    }
}
