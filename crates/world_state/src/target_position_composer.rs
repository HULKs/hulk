use std::f32::consts::PI;

use color_eyre::Result;
use context_attribute::context;
use coordinate_systems::{Field, Ground};
use framework::{AdditionalOutput, MainOutput};
use linear_algebra::{Isometry2, Point2, Pose2, point};
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
    fake_robot_position: Parameter<Vec<Pose2<Field>>, "behavior.voronoi.fake_robot_position">,
    orientation_bias: Parameter<f32, "behavior.voronoi.orientation_bias">,
    voronoi_resolution: Parameter<f32, "behavior.voronoi.grid_resolution">,

    input_points: AdditionalOutput<Vec<Pose2<Field>>, "voronoi.input_points">,
}

#[context]
pub struct MainOutputs {
    pub centroids: MainOutput<Vec<Option<Point2<Field>>>>,
    pub voronoi_grid: MainOutput<Vec<Vec<Point2<Field>>>>,
}

impl TargetPositionComposer {
    pub fn new(_context: CreationContext) -> Result<Self> {
        Ok(Self {})
    }

    pub fn cycle(&mut self, mut context: CycleContext) -> Result<MainOutputs> {
        let mut target_positions = Vec::new();
        let mut voronoi_grid = Vec::new();

        if let Some(ground_to_field) = context.ground_to_field {
            let pose = ground_to_field.as_pose();
            let half_length = context.field_dimensions.length / 2.0;
            let half_width = context.field_dimensions.width / 2.0;

            // TODO: Import the other Robot positions
            let mut sites = vec![pose];
            for fake_position in context.fake_robot_position.into_iter() {
                sites.push(*fake_position);
            }
            context.input_points.fill_if_subscribed(|| sites.clone());

            let resolution = context.voronoi_resolution;
            let mut centroids_sum = vec![(0.0, 0.0); sites.len()];
            let mut cell_counts = vec![0; sites.len()];

            let mut grid_points = vec![Vec::new(); sites.len()];

            let mut x = -half_length;
            while x <= half_length {
                let mut y = -half_width;
                while y <= half_width {
                    let mut best_cost = f32::MAX;
                    let mut owner_index = 0;

                    for (i, site) in sites.iter().enumerate() {
                        let dx = x - site.inner.translation.vector.x;
                        let dy = y - site.inner.translation.vector.y;
                        let distance = (dx * dx + dy * dy).sqrt();

                        let orientation_angle = site.inner.rotation.angle();
                        let angle_to_cell = dy.atan2(dx);

                        let mut angle_diff = (angle_to_cell - orientation_angle).abs();
                        if angle_diff > PI {
                            angle_diff = 2.0 * PI - angle_diff;
                        }

                        let cost = distance + (angle_diff * *context.orientation_bias);

                        if cost < best_cost {
                            best_cost = cost;
                            owner_index = i;
                        }
                    }

                    centroids_sum[owner_index].0 += x;
                    centroids_sum[owner_index].1 += y;
                    cell_counts[owner_index] += 1;

                    grid_points[owner_index].push(point![<Field>, x, y]);

                    y += resolution;
                }
                x += resolution;
            }

            for i in 0..sites.len() {
                if cell_counts[i] > 0 {
                    let cx = centroids_sum[i].0 / cell_counts[i] as f32;
                    let cy = centroids_sum[i].1 / cell_counts[i] as f32;
                    target_positions.push(Some(point![<Field>, cx, cy]));
                    voronoi_grid.push(grid_points[i].clone());
                } else {
                    target_positions.push(None);
                    voronoi_grid.push(Vec::new());
                }
            }
        }

        Ok(MainOutputs {
            centroids: target_positions.into(),
            voronoi_grid: voronoi_grid.into(),
        })
    }
}
