use std::{cmp::Reverse, collections::BinaryHeap};

use color_eyre::Result;
use context_attribute::context;
use coordinate_systems::{Field, Ground};
use framework::{AdditionalOutput, MainOutput};
use hsl_network_messages::PlayerNumber;
use linear_algebra::{Isometry2, Pose2};
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
    dist_buffer: Vec<u32>,
    #[serde(skip)]
    queue_buffer: BinaryHeap<Reverse<(u32, usize, usize)>>,
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
            context.voronoi_parameters.padding * context.voronoi_parameters.grid_resolution,
            context.voronoi_parameters.grid_resolution,
        );

        if let Some(ground_to_field) = context.ground_to_field {
            // TODO: Import the other Robot positions
            let mut sites = vec![(ground_to_field.as_pose(), *context.player_number)];
            for fake_robot in context.voronoi_parameters.fake_robot_position.iter() {
                sites.push(fake_robot.clone());
            }
            context
                .input_points
                .fill_if_subscribed(|| sites.iter().map(|(pose, _)| pose.clone()).collect());

            for obstacle in context.obstacles.iter() {
                let radius = obstacle
                    .radius_at_hip_height
                    .max(obstacle.radius_at_foot_height);
                let center = ground_to_field * obstacle.position;

                if let Some((min_x, max_x, min_y, max_y)) = tile_range_for_bounds(
                    &map,
                    center.x() - radius,
                    center.x() + radius,
                    center.y() - radius,
                    center.y() + radius,
                ) {
                    for y in min_y..=max_y {
                        for x in min_x..=max_x {
                            let index = y * map.width_tiles + x;
                            let grid_point = map.index_to_point(index);
                            if (grid_point - center).norm_squared() <= radius * radius {
                                map.tiles[index] = Ownership::Blocked;
                            }
                        }
                    }
                }
            }

            for rule_obstacle in context.rule_obstacles.iter() {
                match rule_obstacle {
                    RuleObstacle::Circle(circle) => {
                        let radius_squared = circle.radius * circle.radius;
                        if let Some((min_x, max_x, min_y, max_y)) = tile_range_for_bounds(
                            &map,
                            circle.center.x() - circle.radius,
                            circle.center.x() + circle.radius,
                            circle.center.y() - circle.radius,
                            circle.center.y() + circle.radius,
                        ) {
                            for y in min_y..=max_y {
                                for x in min_x..=max_x {
                                    let index = y * map.width_tiles + x;
                                    let grid_point = map.index_to_point(index);
                                    if (grid_point - circle.center).norm_squared() <= radius_squared
                                    {
                                        map.tiles[index] = Ownership::Blocked;
                                    }
                                }
                            }
                        }
                    }
                    RuleObstacle::Rectangle(rectangle) => {
                        if let Some((min_x, max_x, min_y, max_y)) = tile_range_for_bounds(
                            &map,
                            rectangle.min.x(),
                            rectangle.max.x(),
                            rectangle.min.y(),
                            rectangle.max.y(),
                        ) {
                            for y in min_y..=max_y {
                                for x in min_x..=max_x {
                                    let index = y * map.width_tiles + x;
                                    let grid_point = map.index_to_point(index);
                                    if rectangle.contains(grid_point) {
                                        map.tiles[index] = Ownership::Blocked;
                                    }
                                }
                            }
                        }
                    }
                }
            }

            self.dist_buffer.clear();
            if self.dist_buffer.len() != map.tiles.len() {
                self.dist_buffer.resize(map.tiles.len(), u32::MAX);
            } else {
                self.dist_buffer.fill(u32::MAX);
            }
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

fn tile_range_for_bounds(
    map: &VoronoiGrid,
    min_x: f32,
    max_x: f32,
    min_y: f32,
    max_y: f32,
) -> Option<(usize, usize, usize, usize)> {
    if map.width_tiles == 0 || map.height_tiles == 0 {
        return None;
    }

    let tile_min_x = map.min_bound.x();
    let tile_max_x = map.min_bound.x() + map.width_tiles as f32 * map.resolution;
    let tile_min_y = map.min_bound.y();
    let tile_max_y = map.min_bound.y() + map.height_tiles as f32 * map.resolution;

    if max_x < tile_min_x || min_x > tile_max_x || max_y < tile_min_y || min_y > tile_max_y {
        return None;
    }

    let mut min_x_index = ((min_x - tile_min_x) / map.resolution).floor() as isize - 1;
    let mut max_x_index = ((max_x - tile_min_x) / map.resolution).floor() as isize + 1;
    let mut min_y_index = ((min_y - tile_min_y) / map.resolution).floor() as isize - 1;
    let mut max_y_index = ((max_y - tile_min_y) / map.resolution).floor() as isize + 1;

    min_x_index = min_x_index.clamp(0, map.width_tiles as isize - 1);
    max_x_index = max_x_index.clamp(0, map.width_tiles as isize - 1);
    min_y_index = min_y_index.clamp(0, map.height_tiles as isize - 1);
    max_y_index = max_y_index.clamp(0, map.height_tiles as isize - 1);

    Some((
        min_x_index as usize,
        max_x_index as usize,
        min_y_index as usize,
        max_y_index as usize,
    ))
}

fn multi_source_dijkstra(
    map: &mut VoronoiGrid,
    robots: &[(Pose2<Field>, PlayerNumber)],
    dist_buffer: &mut Vec<u32>,
    queue_buffer: &mut BinaryHeap<Reverse<(u32, usize, usize)>>,
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
        if let Some(start_index) = map.nearest_non_blocked_cell_index(robot_pose.position()) {
            if dist_buffer[start_index] > 0 {
                dist_buffer[start_index] = 0;
                map.tiles[start_index] = Ownership::Robot(*player_number);
                queue_buffer.push(Reverse((0, start_index, robot_index)));
            }
        }
    }

    while let Some(Reverse((current_cost, current_index, robot_index))) = queue_buffer.pop() {
        let player_number = robots[robot_index].1;
        if current_cost != dist_buffer[current_index] {
            continue;
        }

        if map.tiles[current_index] != Ownership::Robot(player_number) {
            continue;
        }

        let (sin_h, cos_h) = robot_headings[robot_index];

        let x = current_index % map.width_tiles;
        let y = current_index / map.width_tiles;

        for (dx, dy, step_cost, inv_norm) in NEIGHBORS {
            let nx = x as isize + dx;
            let ny = y as isize + dy;
            if nx < 0 || nx >= map.width_tiles as isize || ny < 0 || ny >= map.height_tiles as isize
            {
                continue;
            }

            let neighbor_index = (ny as usize) * map.width_tiles + (nx as usize);
            if map.tiles[neighbor_index] == Ownership::Blocked {
                continue;
            }

            let rotation_cost = if orientation_bias <= 0.0 {
                0
            } else {
                let dot = (cos_h * dx as f32 + sin_h * dy as f32) * inv_norm;
                let turn_factor = (1.0 - dot.clamp(-1.0, 1.0)) * 0.5;
                (turn_factor * orientation_bias).round() as u32
            };

            let new_cost = current_cost + step_cost + rotation_cost;

            if new_cost < dist_buffer[neighbor_index] {
                dist_buffer[neighbor_index] = new_cost;
                map.tiles[neighbor_index] = Ownership::Robot(player_number);
                queue_buffer.push(Reverse((new_cost, neighbor_index, robot_index)));
            }
        }
    }
}
