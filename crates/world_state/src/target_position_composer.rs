use std::f32::consts::PI;

use color_eyre::Result;
use context_attribute::context;
use coordinate_systems::{Field, Ground};
use framework::{AdditionalOutput, MainOutput};
use linear_algebra::{Isometry2, Point2, point};
use serde::{Deserialize, Serialize};
use types::field_dimensions::FieldDimensions;

#[derive(Deserialize, Serialize)]
pub struct TargetPositionComposer {}

#[context]
pub struct CreationContext {}

#[context]
pub struct CycleContext {
    ground_to_field: Input<Option<Isometry2<Ground, Field>>, "ground_to_field?">,
    field_dimensions: Parameter<FieldDimensions, "field_dimensions">,
    fake_robot_position: Parameter<Vec<Point2<Field>>, "behavior.voronoi.fake_robot_position">,
    orientation_bias: Parameter<f32, "behavior.voronoi.orientation_bias">,

    input_points: AdditionalOutput<Vec<Point2<Field>>, "voronoi.input_points">,
}

#[context]
pub struct MainOutputs {
    pub centroids: MainOutput<Vec<Option<Point2<Field>>>>,
    pub voronoi_cells: MainOutput<Vec<Vec<Point2<Field>>>>,
}

#[derive(Clone, Copy)]
struct VoronoiSite {
    x: f32,
    y: f32,
    forward_x: f32,
    forward_y: f32,
}

impl TargetPositionComposer {
    pub fn new(_context: CreationContext) -> Result<Self> {
        Ok(Self {})
    }

    pub fn cycle(&mut self, mut context: CycleContext) -> Result<MainOutputs> {
        let mut target_positions = Vec::new();
        let mut voronoi_cells = Vec::new();

        if let Some(ground_to_field) = context.ground_to_field {
            let pose = ground_to_field.as_pose();
            let half_length = context.field_dimensions.length / 2.0;
            let half_width = context.field_dimensions.width / 2.0;
            let orientation = pose.orientation().angle();

            // TODO: Import the other Robot positions
            let mut sites = vec![VoronoiSite {
                x: pose.position().x(),
                y: pose.position().y(),
                forward_x: orientation.cos(),
                forward_y: orientation.sin(),
            }];
            for fake_position in context.fake_robot_position.into_iter() {
                sites.push(VoronoiSite {
                    x: fake_position.x(),
                    y: fake_position.y(),
                    forward_x: 1.0,
                    forward_y: 0.0,
                });
            }
            context.input_points.fill_if_subscribed(|| {
                sites
                    .iter()
                    .map(|site| point![<Field>, site.x, site.y])
                    .collect::<Vec<Point2<Field>>>()
            });

            let resolution = 0.1; // 10cm grid
            let mut centroids_sum = vec![(0.0, 0.0); sites.len()];
            let mut cell_counts = vec![0; sites.len()];

            // To visualize we can keep the grid points as the "cells"
            // (Note: Downstream visualization might try to draw a polygon line through these.
            // If it looks like a zigzag mess, you might want to return empty vectors here instead).
            let mut grid_points = vec![Vec::new(); sites.len()];

            let mut x = -half_length;
            while x <= half_length {
                let mut y = -half_width;
                while y <= half_width {
                    let mut best_cost = f32::MAX;
                    let mut owner_index = 0;

                    for (i, site) in sites.iter().enumerate() {
                        let dx = x - site.x;
                        let dy = y - site.y;
                        let distance = (dx * dx + dy * dy).sqrt();

                        // Calculate Time-to-Reach cost: Distance + Turn Penalty
                        let cost = if site.forward_x.abs() > f32::EPSILON
                            || site.forward_y.abs() > f32::EPSILON
                        {
                            let angle_to_cell = dy.atan2(dx);
                            let robot_angle = site.forward_y.atan2(site.forward_x);

                            let mut angle_diff = (angle_to_cell - robot_angle).abs();
                            if angle_diff > PI {
                                angle_diff = 2.0 * PI - angle_diff;
                            }

                            distance + (angle_diff * *context.orientation_bias)
                        } else {
                            distance
                        };

                        if cost < best_cost {
                            best_cost = cost;
                            owner_index = i;
                        }
                    }

                    // Assign the pixel to the winning robot
                    centroids_sum[owner_index].0 += x;
                    centroids_sum[owner_index].1 += y;
                    cell_counts[owner_index] += 1;

                    // Only collect visualization points every few cells to save memory/rendering if needed
                    grid_points[owner_index].push(point![<Field>, x, y]);

                    y += resolution;
                }
                x += resolution;
            }

            // Calculate final centroids
            for i in 0..sites.len() {
                if cell_counts[i] > 0 {
                    let cx = centroids_sum[i].0 / cell_counts[i] as f32;
                    let cy = centroids_sum[i].1 / cell_counts[i] as f32;
                    target_positions.push(Some(point![<Field>, cx, cy]));
                    voronoi_cells.push(grid_points[i].clone());
                } else {
                    target_positions.push(None);
                    voronoi_cells.push(Vec::new());
                }
            }
        }

        Ok(MainOutputs {
            centroids: target_positions.into(),
            voronoi_cells: voronoi_cells.into(),
        })
    }
}
