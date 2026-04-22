use std::{cmp::Reverse, collections::BinaryHeap};

use color_eyre::Result;
use context_attribute::context;
use coordinate_systems::{Field, Ground};
use framework::{AdditionalOutput, MainOutput};
use hsl_network_messages::PlayerNumber;
use linear_algebra::{Isometry2, Point2, Pose2};
use serde::{Deserialize, Serialize};
use types::{
    field_dimensions::FieldDimensions,
    obstacles::Obstacle,
    parameters::VoronoiParameters,
    rule_obstacles::RuleObstacle,
    voronoi::{Map, Ownership},
};

const STRAIGHT_COST: u32 = 10;
const DIAGONAL_COST: u32 = 14;
const PADDING_FACTOR: f32 = 4.0;

#[derive(Deserialize, Serialize)]
pub struct TargetPositionComposer {}

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
    pub centroids: MainOutput<Vec<Option<Point2<Field>>>>,
    pub voronoi_grid: MainOutput<Vec<(Point2<Field>, Ownership)>>,
}

impl TargetPositionComposer {
    pub fn new(_context: CreationContext) -> Result<Self> {
        Ok(Self {})
    }

    pub fn cycle(&mut self, mut context: CycleContext) -> Result<MainOutputs> {
        let mut ownership_grid = Vec::new();

        if let Some(ground_to_field) = context.ground_to_field {
            // TODO: Import the other Robot positions
            let mut sites = vec![(ground_to_field.as_pose(), *context.player_number)];
            for fake_robot in context
                .voronoi_parameters
                .fake_robot_position
                .clone()
                .into_iter()
            {
                sites.push(fake_robot);
            }
            context
                .input_points
                .fill_if_subscribed(|| sites.iter().map(|(pose, _)| pose.clone()).collect());

            let mut map = Map::new(
                context.field_dimensions.length,
                context.field_dimensions.width,
                PADDING_FACTOR * context.voronoi_parameters.grid_resolution,
                context.voronoi_parameters.grid_resolution,
            );

            for index in 0..map.tiles.len() {
                let grid_point = map.index_to_point(index);
                if point_is_inside_obstacle(grid_point, ground_to_field, &context.obstacles)
                    || point_is_inside_rule_obstacle(grid_point, &context.rule_obstacles)
                {
                    map.tiles[index] = Ownership::Blocked
                }
            }

            multi_source_dijkstra(&mut map, &sites);

            ownership_grid.reserve(map.tiles.len());
            for index in 0..map.tiles.len() {
                ownership_grid.push((map.index_to_point(index), map.tiles[index]));
            }
        }

        Ok(MainOutputs {
            centroids: vec![None].into(),
            voronoi_grid: ownership_grid.into(),
        })
    }
}

fn point_is_inside_obstacle(
    point: Point2<Field>,
    ground_to_field: &Isometry2<Ground, Field>,
    obstacles: &[Obstacle],
) -> bool {
    obstacles
        .iter()
        .any(|obstacle| obstacle.contains_point(ground_to_field.inverse() * point))
}

fn point_is_inside_rule_obstacle(point: Point2<Field>, rule_obstacles: &[RuleObstacle]) -> bool {
    rule_obstacles
        .iter()
        .any(|rule_obstacle| rule_obstacle.contains(point))
}

fn multi_source_dijkstra(map: &mut Map, robots: &[(Pose2<Field>, PlayerNumber)]) {
    if map.width_tiles == 0
        || map.height_tiles == 0
        || map.width_tiles * map.height_tiles != map.tiles.len()
    {
        return;
    }

    let mut dist = vec![u32::MAX; map.tiles.len()];
    let mut queue = BinaryHeap::new();

    for (robot_pose, player_number) in robots.iter() {
        if let Some(start_index) = map.nearest_non_blocked_cell_index(robot_pose.position()) {
            if dist[start_index] > 0 {
                dist[start_index] = 0;
                map.tiles[start_index] = Ownership::Robot(*player_number);
                queue.push(Reverse((0, start_index, *player_number)));
            }
        }
    }

    while let Some(Reverse((current_cost, current_index, player_number))) = queue.pop() {
        if current_cost > dist[current_index] {
            continue;
        }

        if map.tiles[current_index] != Ownership::Robot(player_number) {
            continue;
        }

        for (neighbor_index, step_cost) in get_neighbor_with_cost(map, current_index) {
            let new_cost = current_cost + step_cost;

            if new_cost < dist[neighbor_index] {
                dist[neighbor_index] = new_cost;
                map.tiles[neighbor_index] = Ownership::Robot(player_number);
                queue.push(Reverse((new_cost, neighbor_index, player_number)));
            }
        }
    }
}

fn get_neighbor_with_cost(map: &Map, index: usize) -> Vec<(usize, u32)> {
    let mut neighbors_with_cost = Vec::new();
    let x = index % map.width_tiles;
    let y = index / map.width_tiles;

    for dx in -1..=1 {
        for dy in -1..=1 {
            if dx == 0 && dy == 0 {
                continue;
            }
            let nx = x as isize + dx;
            let ny = y as isize + dy;
            if nx >= 0 && nx < map.width_tiles as isize && ny >= 0 && ny < map.height_tiles as isize
            {
                let neighbor_index = (ny as usize) * map.width_tiles + (nx as usize);
                if map.tiles[neighbor_index] != Ownership::Blocked {
                    if dx.abs() + dy.abs() == 2 {
                        neighbors_with_cost.push((neighbor_index, DIAGONAL_COST)); // Diagonal move for symplicity
                    } else {
                        neighbors_with_cost.push((neighbor_index, STRAIGHT_COST)); // Straight move
                    }
                }
            }
        }
    }

    neighbors_with_cost
}
