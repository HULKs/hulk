use std::{cmp::Reverse, collections::BinaryHeap};

use color_eyre::Result;
use context_attribute::context;
use coordinate_systems::{Field, Ground};
use framework::{AdditionalOutput, MainOutput};
use hsl_network_messages::PlayerNumber;
use linear_algebra::{Isometry2, Point2, Pose2};
use ordered_float::NotNan;
use serde::{Deserialize, Serialize};
use types::{
    field_dimensions::FieldDimensions,
    obstacles::Obstacle,
    parameters::VoronoiParameters,
    rule_obstacles::RuleObstacle,
    voronoi::{NEIGHBORS, Ownership, VoronoiGrid},
};

#[derive(Deserialize, Serialize)]
pub struct TargetPositionComposer {
    #[serde(skip)]
    dist_buffer: Vec<f32>,
    #[serde(skip)]
    queue_buffer: BinaryHeap<Reverse<(NotNan<f32>, usize, usize)>>,
}

#[context]
pub struct CreationContext {}

#[context]
pub struct CycleContext {
    ground_to_field: Input<Option<Isometry2<Ground, Field>>, "ground_to_field?">,
    rule_obstacles: Input<Vec<RuleObstacle>, "rule_obstacles">,
    obstacles: Input<Vec<Obstacle>, "obstacles">,

    field_dimensions: Parameter<FieldDimensions, "field_dimensions">,
    voronoi_parameters: Parameter<VoronoiParameters, "behavior.voronoi">,
    player_number: Parameter<PlayerNumber, "player_number">,

    input_points: AdditionalOutput<Vec<Pose2<Field>>, "voronoi.input_points">,
}

#[context]
pub struct MainOutputs {
    pub voronoi_grid: MainOutput<VoronoiGrid>,
}

impl TargetPositionComposer {
    pub fn new(_context: CreationContext) -> Result<Self> {
        Ok(Self {
            dist_buffer: Vec::new(),
            queue_buffer: BinaryHeap::new(),
        })
    }

    pub fn cycle(&mut self, mut context: CycleContext) -> Result<MainOutputs> {
        let mut map = VoronoiGrid::new(
            context.field_dimensions.length,
            context.field_dimensions.width,
            context.voronoi_parameters.padding,
            context.voronoi_parameters.grid_resolution,
        );

        if let Some(ground_to_field) = context.ground_to_field {
            // TODO: Import the other Robot positions
            let mut sites = vec![(ground_to_field.as_pose(), *context.player_number)];
            for fake_robot in context.voronoi_parameters.fake_robot_position.iter() {
                sites.push(*fake_robot);
            }
            context
                .input_points
                .fill_if_subscribed(|| sites.iter().map(|(pose, _)| *pose).collect());

            for obstacle in context.obstacles.iter() {
                let radius = obstacle
                    .radius_at_hip_height
                    .max(obstacle.radius_at_foot_height);
                let center = ground_to_field * obstacle.position;

                let radius_squared = radius * radius;
                rasterize_bounds(
                    &mut map,
                    center.x() - radius,
                    center.x() + radius,
                    center.y() - radius,
                    center.y() + radius,
                    |grid_point| (grid_point - center).norm_squared() <= radius_squared,
                );
            }

            for rule_obstacle in context.rule_obstacles.iter() {
                match rule_obstacle {
                    RuleObstacle::Circle(circle) => {
                        let radius_squared = circle.radius * circle.radius;
                        rasterize_bounds(
                            &mut map,
                            circle.center.x() - circle.radius,
                            circle.center.x() + circle.radius,
                            circle.center.y() - circle.radius,
                            circle.center.y() + circle.radius,
                            |grid_point| {
                                (grid_point - circle.center).norm_squared() <= radius_squared
                            },
                        );
                    }
                    RuleObstacle::Rectangle(rectangle) => {
                        rasterize_bounds(
                            &mut map,
                            rectangle.min.x(),
                            rectangle.max.x(),
                            rectangle.min.y(),
                            rectangle.max.y(),
                            |grid_point| rectangle.contains(grid_point),
                        );
                    }
                }
            }

            if self.dist_buffer.len() != map.tiles.len() {
                self.dist_buffer.resize(map.tiles.len(), f32::INFINITY);
            }
            self.dist_buffer.fill(f32::INFINITY);
            self.queue_buffer.clear();

            multi_source_dijkstra(
                &mut map,
                &sites,
                &mut self.dist_buffer,
                &mut self.queue_buffer,
                context.voronoi_parameters.orientation_bias,
            );
        }

        Ok(MainOutputs {
            voronoi_grid: map.into(),
        })
    }
}

fn rasterize_bounds(
    map: &mut VoronoiGrid,
    min_x: f32,
    max_x: f32,
    min_y: f32,
    max_y: f32,
    mut contains: impl FnMut(Point2<Field>) -> bool,
) {
    let Some((min_x, max_x, min_y, max_y)) = map.tile_range_for_bounds(min_x, max_x, min_y, max_y)
    else {
        return;
    };

    for y in min_y..=max_y {
        for x in min_x..=max_x {
            let index = y * map.width_tiles + x;
            let grid_point = map.index_to_point(index);
            if contains(grid_point) {
                map.tiles[index] = Ownership::Blocked;
            }
        }
    }
}

fn multi_source_dijkstra(
    map: &mut VoronoiGrid,
    robots: &[(Pose2<Field>, PlayerNumber)],
    dist_buffer: &mut [f32],
    queue_buffer: &mut BinaryHeap<Reverse<(NotNan<f32>, usize, usize)>>,
    orientation_bias: f32,
) {
    if map.width_tiles == 0
        || map.height_tiles == 0
        || map.width_tiles * map.height_tiles != map.tiles.len()
    {
        return;
    }

    let robot_headings: Vec<(f32, f32)> = robots
        .iter()
        .map(|(pose, _)| pose.orientation().angle().sin_cos())
        .collect();

    for (robot_index, (robot_pose, player_number)) in robots.iter().enumerate() {
        if let Some(start_index) = map.nearest_non_blocked_cell_index(robot_pose.position())
            && dist_buffer[start_index] > 0.0
        {
            dist_buffer[start_index] = 0.0;
            map.tiles[start_index] = Ownership::Robot(*player_number);
            queue_buffer.push(Reverse((
                NotNan::new(0.0).unwrap(),
                start_index,
                robot_index,
            )));
        }
    }

    while let Some(Reverse((current_cost, current_index, robot_index))) = queue_buffer.pop() {
        let current_cost = current_cost.into_inner();
        let player_number = robots[robot_index].1;
        if current_cost > dist_buffer[current_index] {
            continue;
        }

        let (sin_h, cos_h) = robot_headings[robot_index];

        let x = current_index % map.width_tiles;
        let y = current_index / map.width_tiles;

        for neighbor in NEIGHBORS {
            let nx = x as isize + neighbor.dx;
            let ny = y as isize + neighbor.dy;
            if nx < 0 || nx >= map.width_tiles as isize || ny < 0 || ny >= map.height_tiles as isize
            {
                continue;
            }

            let neighbor_index = (ny as usize) * map.width_tiles + (nx as usize);
            if map.tiles[neighbor_index] == Ownership::Blocked {
                continue;
            }

            let rotation_cost = if orientation_bias <= 0.0 {
                0.0
            } else {
                let dot =
                    (cos_h * neighbor.dx as f32 + sin_h * neighbor.dy as f32) * neighbor.inv_norm;
                let turn_factor = (1.0 - dot.clamp(-1.0, 1.0)) * 0.5;
                turn_factor * orientation_bias
            };

            let new_cost = current_cost + neighbor.step_cost + rotation_cost;

            if new_cost < dist_buffer[neighbor_index] {
                dist_buffer[neighbor_index] = new_cost;
                map.tiles[neighbor_index] = Ownership::Robot(player_number);
                queue_buffer.push(Reverse((
                    NotNan::new(new_cost).unwrap(),
                    neighbor_index,
                    robot_index,
                )));
            }
        }
    }
}
