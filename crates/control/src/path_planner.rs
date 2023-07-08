use color_eyre::{eyre::eyre, Result};
use nalgebra::{distance, point, vector, Isometry2, Point2, UnitComplex};
use ordered_float::NotNan;
use smallvec::SmallVec;

use types::{
    Arc, Circle, FieldDimensions, LineSegment, MotionCommand, Obstacle, Orientation, PathObstacle,
    PathObstacleShape, PathSegment, RuleObstacle,
};

use crate::a_star::{a_star_search, DynamicMap};

#[derive(Debug, Clone)]
pub struct PathNode {
    pub position: Point2<f32>,
    pub obstacle: Option<usize>,
    pub pair_node: Option<usize>,
    pub allow_local_exits: bool,
}

impl From<Point2<f32>> for PathNode {
    fn from(position: Point2<f32>) -> Self {
        Self {
            position,
            obstacle: None,
            pair_node: None,
            allow_local_exits: false,
        }
    }
}

#[derive(Debug, Default)]
pub struct PathPlanner {
    /// The first node is always the start, the second the destination
    pub nodes: Vec<PathNode>,
    pub obstacles: Vec<PathObstacle>,
    pub last_orientation: Option<UnitComplex<f32>>,
    pub rotation_penalty_factor: f32,
}

impl PathPlanner {
    pub fn from_last_motion(
        last_motion_command: &MotionCommand,
        rotation_penalty_factor: f32,
    ) -> Self {
        let last_orientation = match last_motion_command {
            MotionCommand::Walk {
                orientation_mode,
                path,
                ..
            } => match orientation_mode.clone() {
                types::OrientationMode::AlignWithPath => path.first().map(|segment| {
                    let direction = match segment {
                        PathSegment::LineSegment(line_segment) => line_segment.1.coords,
                        PathSegment::Arc(arc, orientation) => orientation
                            .rotate_vector_90_degrees(arc.start - arc.circle.center)
                            .normalize(),
                    };
                    if direction.norm_squared() < f32::EPSILON {
                        UnitComplex::identity()
                    } else {
                        let normalized_direction = direction.normalize();
                        UnitComplex::from_cos_sin_unchecked(
                            normalized_direction.x,
                            normalized_direction.y,
                        )
                    }
                }),
                types::OrientationMode::Override(orientation) => Some(orientation),
            },
            _ => None,
        };

        Self {
            last_orientation,
            rotation_penalty_factor,
            ..Default::default()
        }
    }

    pub fn with_obstacles(&mut self, obstacles: &[Obstacle], own_robot_radius: f32) {
        let new_obstacles = obstacles.iter().map(|obstacle| {
            let position = obstacle.position;
            let radius = obstacle.radius_at_hip_height + own_robot_radius;
            PathObstacle::from(PathObstacleShape::Circle(Circle {
                center: position,
                radius,
            }))
        });

        self.obstacles.extend(new_obstacles);
    }

    pub fn with_rule_obstacles(
        &mut self,
        field_to_robot: Isometry2<f32>,
        rule_obstacles: &[RuleObstacle],
        own_robot_radius: f32,
    ) {
        let new_obstacles = rule_obstacles
            .iter()
            .flat_map(|rule_obstacle| match rule_obstacle {
                RuleObstacle::Rectangle(rectangle) => {
                    let bottom_left = field_to_robot * rectangle.min;
                    let top_right = field_to_robot * rectangle.max;
                    let top_left = field_to_robot * point![rectangle.min.x, rectangle.max.y];
                    let bottom_right = field_to_robot * point![rectangle.max.x, rectangle.min.y];
                    vec![
                        PathObstacle::from(Circle::new(bottom_left, own_robot_radius)),
                        PathObstacle::from(Circle::new(bottom_right, own_robot_radius)),
                        PathObstacle::from(Circle::new(top_left, own_robot_radius)),
                        PathObstacle::from(Circle::new(top_right, own_robot_radius)),
                        PathObstacle::from(LineSegment::new(bottom_left, bottom_right)),
                        PathObstacle::from(LineSegment::new(bottom_right, top_right)),
                        PathObstacle::from(LineSegment::new(top_right, top_left)),
                        PathObstacle::from(LineSegment::new(top_left, bottom_left)),
                    ]
                }
                RuleObstacle::Circle(circle) => {
                    vec![PathObstacle::from(Circle::new(
                        field_to_robot * circle.center,
                        circle.radius + own_robot_radius,
                    ))]
                }
            });
        self.obstacles.extend(new_obstacles);
    }

    pub fn with_ball(
        &mut self,
        ball_position: Point2<f32>,
        ball_radius: f32,
        own_robot_radius: f32,
    ) {
        let shape = PathObstacleShape::Circle(Circle {
            center: ball_position,
            radius: ball_radius + own_robot_radius,
        });
        self.obstacles.push(PathObstacle::from(shape));
    }

    pub fn with_field_borders(
        &mut self,
        robot_to_field: Isometry2<f32>,
        field_length: f32,
        field_width: f32,
        margin: f32,
        distance_weight: f32,
    ) -> &mut Self {
        let own_position = robot_to_field * Point2::origin();

        let distance_to_left_field_border = (field_length / 2.0 - -own_position.x).max(0.0);
        let distance_to_right_field_border = (field_length / 2.0 - own_position.x).max(0.0);
        let distance_to_lower_field_border = (field_width / 2.0 - -own_position.y).max(0.0);
        let distance_to_upper_field_border = (field_width / 2.0 - own_position.y).max(0.0);

        let field_to_robot = robot_to_field.inverse();
        let x = field_length / 2.0 + margin;
        let y = field_width / 2.0 + margin;
        let bottom_right = field_to_robot * point![x, -y];
        let top_right = field_to_robot * point![x, y];
        let bottom_left = field_to_robot * point![-x, -y];
        let top_left = field_to_robot * point![-x, y];

        let line_segments = [
            LineSegment(bottom_left, top_left).translate(
                &(field_to_robot
                    * vector![
                        -distance_to_left_field_border.powf(2.0) * distance_weight,
                        0.0
                    ]),
            ),
            LineSegment(top_left, top_right).translate(
                &(field_to_robot
                    * vector![
                        0.0,
                        distance_to_upper_field_border.powf(2.0) * distance_weight
                    ]),
            ),
            LineSegment(top_right, bottom_right).translate(
                &(field_to_robot
                    * vector![
                        distance_to_right_field_border.powf(2.0) * distance_weight,
                        0.0
                    ]),
            ),
            LineSegment(bottom_right, bottom_left).translate(
                &(field_to_robot
                    * vector![
                        0.0,
                        -distance_to_lower_field_border.powf(2.0) * distance_weight
                    ]),
            ),
        ];

        self.obstacles.extend(
            line_segments.into_iter().map(|line_segment| {
                PathObstacle::from(PathObstacleShape::LineSegment(line_segment))
            }),
        );

        self
    }

    pub fn with_goal_support_structures(
        &mut self,
        field_to_robot: Isometry2<f32>,
        field_dimensions: &FieldDimensions,
    ) {
        let goal_post_x = field_dimensions.length / 2.0 + field_dimensions.goal_post_diameter / 2.0
            - field_dimensions.line_width / 2.0;
        let goal_post_y =
            field_dimensions.goal_inner_width / 2.0 + field_dimensions.goal_post_diameter / 2.0;
        let field_border_x = field_dimensions.length / 2.0 + field_dimensions.border_strip_width;

        let post_to_border = |x_sign: f32, y_sign: f32| {
            LineSegment(
                field_to_robot * point![x_sign * goal_post_x, y_sign * goal_post_y],
                field_to_robot * point![x_sign * field_border_x, y_sign * goal_post_y],
            )
        };

        let line_segments = [
            post_to_border(1.0, 1.0),
            post_to_border(-1.0, 1.0),
            post_to_border(1.0, -1.0),
            post_to_border(-1.0, -1.0),
        ];

        self.obstacles.extend(
            line_segments.into_iter().map(|line_segment| {
                PathObstacle::from(PathObstacleShape::LineSegment(line_segment))
            }),
        );
    }

    fn generate_start_destination_tangents(&mut self) {
        let direct_path = LineSegment(self.nodes[0].position, self.nodes[1].position);
        let direct_path_blocked = self
            .obstacles
            .iter()
            .any(|obstacle| obstacle.shape.intersects_line_segment(direct_path));

        if !direct_path_blocked {
            self.nodes[0].pair_node = Some(1);
            self.nodes[1].pair_node = Some(0);
            return;
        }

        for index in 0..self.obstacles.len() {
            let circle = match self.obstacles[index].shape {
                PathObstacleShape::Circle(circle) => circle,
                _ => continue,
            };
            if let Some(tangents) = circle.tangents_with_point(self.nodes[0].position) {
                self.add_tangent_between_point_and_obstacle(tangents.0, 0, index);
                self.add_tangent_between_point_and_obstacle(tangents.1, 0, index);
            };
            if let Some(tangents) = circle.tangents_with_point(self.nodes[1].position) {
                self.add_tangent_between_point_and_obstacle(tangents.0, 1, index);
                self.add_tangent_between_point_and_obstacle(tangents.1, 1, index);
            };
        }
    }

    pub fn plan(
        &mut self,
        mut start: Point2<f32>,
        mut destination: Point2<f32>,
    ) -> Result<Option<Vec<PathSegment>>> {
        let closest_circle = self
            .obstacles
            .iter()
            .filter_map(|obstacle| obstacle.shape.as_circle())
            .filter(|circle| distance(&circle.center, &start) <= circle.radius)
            .min_by_key(|circle| NotNan::new(circle.center.coords.norm_squared()).unwrap());
        if let Some(circle) = closest_circle {
            let to_start = start - circle.center;
            let safety_radius = circle.radius * 1.1;
            start += to_start.normalize() * (safety_radius - to_start.norm());
        }

        let closest_circle = self
            .obstacles
            .iter()
            .filter_map(|obstacle| obstacle.shape.as_circle())
            .filter(|circle| distance(&circle.center, &destination) <= circle.radius)
            .min_by_key(|circle| NotNan::new(circle.center.coords.norm_squared()).unwrap());
        if let Some(circle) = closest_circle {
            let to_destination = destination - circle.center;
            let safety_radius = circle.radius * 1.1;
            destination += to_destination.normalize() * (safety_radius - to_destination.norm());
        }

        for circle in self
            .obstacles
            .iter_mut()
            .filter_map(|obstacle| obstacle.shape.as_circle_mut())
        {
            let to_start = start - circle.center;
            let safety_radius = circle.radius * 1.1;
            if to_start.norm_squared() <= safety_radius.powi(2) {
                circle.radius -= safety_radius - to_start.norm();
            }

            let to_destination = destination - circle.center;
            let safety_radius = circle.radius * 1.1;
            if to_destination.norm_squared() <= safety_radius.powi(2) {
                circle.radius -= safety_radius - to_destination.norm();
            }
        }

        self.nodes = vec![PathNode::from(start), PathNode::from(destination)];

        self.generate_start_destination_tangents();

        let navigation_path = a_star_search(0, 1, self);

        if !navigation_path.success {
            return Ok(None);
        }

        let mut previous_node_index = 0;
        let path_segments = navigation_path
            .steps
            .windows(2)
            .map(|indices| -> Result<PathSegment> {
                let previous_node = &self.nodes[previous_node_index];
                previous_node_index = indices[0];
                let current_node = &self.nodes[indices[0]];
                let next_node = &self.nodes[indices[1]];
                match (current_node.obstacle, next_node.obstacle) {
                    (Some(current_obstacle_index), Some(next_obstacle_index))
                        if current_obstacle_index == next_obstacle_index =>
                    {
                        let &circle = self.obstacles[current_obstacle_index]
                            .shape
                            .as_circle()
                            .ok_or_else(|| eyre!("obstacle from path node was not a circle"))?;
                        Ok(PathSegment::Arc(
                            Arc {
                                circle,
                                start: current_node.position,
                                end: next_node.position,
                            },
                            LineSegment(previous_node.position, current_node.position)
                                .get_orientation(circle.center),
                        ))
                    }
                    _ => Ok(PathSegment::LineSegment(LineSegment(
                        current_node.position,
                        next_node.position,
                    ))),
                }
            })
            .collect::<Result<Vec<_>>>()
            .map(Some);

        path_segments
    }

    fn add_tangent_between_point_and_obstacle(
        &mut self,
        tangent: LineSegment,
        point_index: usize,
        obstacle_index: usize,
    ) {
        if self.obstacles.iter().enumerate().any(|(index, obstacle)| {
            obstacle.shape.intersects_line_segment(tangent) && index != obstacle_index
        }) {
            return;
        }

        let node1 = PathNode {
            position: tangent.0,
            obstacle: Some(obstacle_index),
            pair_node: Some(point_index),
            allow_local_exits: false,
        };

        self.nodes.push(node1);
        self.obstacles[obstacle_index]
            .nodes
            .push(self.nodes.len() - 1);
    }

    fn add_tangent(
        &mut self,
        tangent: LineSegment,
        obstacle1_index: usize,
        obstacle2_index: usize,
    ) {
        if self.obstacles.iter().enumerate().any(|(index, obstacle)| {
            index != obstacle1_index
                && index != obstacle2_index
                && obstacle.shape.intersects_line_segment(tangent)
        }) {
            return;
        }

        let node1 = PathNode {
            position: tangent.0,
            obstacle: Some(obstacle1_index),
            pair_node: Some(self.nodes.len() + 1),
            allow_local_exits: false,
        };
        let node2 = PathNode {
            position: tangent.1,
            obstacle: Some(obstacle2_index),
            pair_node: Some(self.nodes.len()),
            allow_local_exits: false,
        };

        self.nodes.push(node1);
        self.obstacles[obstacle1_index]
            .nodes
            .push(self.nodes.len() - 1);

        self.nodes.push(node2);
        self.obstacles[obstacle2_index]
            .nodes
            .push(self.nodes.len() - 1);
    }

    fn get_orientation_to_obstacle(&self, node: usize, obstacle_index: usize) -> Orientation {
        let pair_node = self.nodes[node].pair_node.unwrap();
        let tangent = LineSegment(self.nodes[pair_node].position, self.nodes[node].position);

        match &self.obstacles[obstacle_index].shape {
            PathObstacleShape::Circle(circle) => tangent.get_orientation(circle.center),
            PathObstacleShape::LineSegment(_) => panic!("LineSegment not implemented"),
        }
    }

    fn populate_obstacle(&mut self, obstacle_index: usize) {
        if self.obstacles[obstacle_index].populated_connections.len() == self.obstacles.len() - 1 {
            return;
        }
        for other_index in 0..self.obstacles.len() {
            if obstacle_index == other_index {
                continue;
            };
            if !self.obstacles[obstacle_index]
                .populated_connections
                .insert(other_index)
            {
                continue;
            }
            if !self.obstacles[other_index]
                .populated_connections
                .insert(obstacle_index)
            {
                continue;
            };
            let circle1 = match &self.obstacles[obstacle_index].shape {
                PathObstacleShape::Circle(circle) => circle,
                _ => continue,
            };
            let circle2 = match &self.obstacles[other_index].shape {
                PathObstacleShape::Circle(circle) => circle,
                _ => continue,
            };
            if let Some(tangents) = circle1.tangents_with_circle(*circle2) {
                self.add_tangent(tangents.outer.0, obstacle_index, other_index);
                self.add_tangent(tangents.outer.1, obstacle_index, other_index);
                if let Some(inner_tangents) = tangents.inner {
                    self.add_tangent(inner_tangents.0, obstacle_index, other_index);
                    self.add_tangent(inner_tangents.1, obstacle_index, other_index);
                };
            };
        }
    }
}

impl DynamicMap for PathPlanner {
    fn get_pathing_distance(&self, index1: usize, index2: usize) -> f32 {
        let direction = self.nodes[index2].position - self.nodes[index1].position;
        let mut distance = direction.norm();

        if index1 == 0 && distance > 0.0 {
            if let Some(current_rotation) = self.last_orientation {
                let normalized_direction = direction.normalize();
                let rotation = current_rotation.rotation_to(&UnitComplex::from_cos_sin_unchecked(
                    normalized_direction.x,
                    normalized_direction.y,
                ));

                distance += rotation.angle().abs() * self.rotation_penalty_factor;
            }
        }

        distance
    }

    fn get_available_exits(&mut self, index: usize) -> SmallVec<[(usize, f32); 10]> {
        let mut vector = SmallVec::new();
        if let Some(pair_index) = self.nodes[index].pair_node {
            vector.push((pair_index, self.get_pathing_distance(index, pair_index)));
            self.nodes[pair_index].allow_local_exits = true;
        } else {
            for pair_index in 0..self.nodes.len() {
                if self.nodes[pair_index].pair_node == Some(index) {
                    vector.push((pair_index, self.get_pathing_distance(index, pair_index)));
                    self.nodes[pair_index].allow_local_exits = true;
                }
            }
        }
        if let (Some(obstacle_index), true) = (
            self.nodes[index].obstacle,
            self.nodes[index].allow_local_exits,
        ) {
            self.populate_obstacle(obstacle_index);
            let orientation = self.get_orientation_to_obstacle(index, obstacle_index);
            for other_node in &self.obstacles[obstacle_index].nodes {
                if *other_node != index {
                    let other_orientation =
                        self.get_orientation_to_obstacle(*other_node, obstacle_index);
                    if orientation != other_orientation {
                        let &circle = self.obstacles[obstacle_index]
                            .shape
                            .as_circle()
                            .expect("ObstacleShape must be a circle");
                        let arc = Arc::new(
                            circle,
                            self.nodes[index].position,
                            self.nodes[*other_node].position,
                        );
                        if self
                            .obstacles
                            .iter()
                            .enumerate()
                            .filter(|(index, _)| *index != obstacle_index)
                            .all(|(_, obstacle)| !obstacle.shape.overlaps_arc(arc, orientation))
                        {
                            vector.push((*other_node, arc.length(orientation)));
                        }
                    }
                }
            }
        }

        vector
    }
}

#[cfg(test)]
mod tests {
    use std::f32::consts::PI;

    use approx::assert_relative_eq;
    use nalgebra::point;

    use super::*;
    use types::Circle;

    fn run_test_scenario(
        start: Point2<f32>,
        end: Point2<f32>,
        map: &mut PathPlanner,
        expected_segments: &[PathSegment],
        expected_cost: f32,
    ) {
        let path = map
            .plan(start, end)
            .expect("Path error")
            .expect("Path was none");

        println!("Map {map:#?}");
        println!(
            "Total cost: {:?}",
            path.iter().map(|segment| segment.length()).sum::<f32>()
        );

        assert_relative_eq!(path.as_slice(), expected_segments, epsilon = 0.01);
        assert_relative_eq!(
            path.iter().map(|segment| segment.length()).sum::<f32>(),
            expected_cost,
            epsilon = 0.01
        );
    }

    #[test]
    fn direct_path() {
        run_test_scenario(
            point![-2.0, 0.0],
            point![2.0, 0.0],
            &mut PathPlanner::default(),
            &[PathSegment::LineSegment(LineSegment(
                point![-2.0, 0.0],
                point![2.0, 0.0],
            ))],
            4.0,
        );
    }

    #[test]
    fn direct_path_with_obstacle() {
        let mut planner = PathPlanner::default();
        planner.with_obstacles(&[Obstacle::ball(point![0.0, 2.0], 1.0)], 0.0);
        run_test_scenario(
            point![-2.0, 0.0],
            point![2.0, 0.0],
            &mut planner,
            &[PathSegment::LineSegment(LineSegment(
                point![-2.0, 0.0],
                point![2.0, 0.0],
            ))],
            4.0,
        );
    }

    #[test]
    fn path_with_circle() {
        let mut planner = PathPlanner::default();
        planner.with_obstacles(&[Obstacle::ball(point![0.0, 0.0], 1.0)], 0.0);
        run_test_scenario(
            point![-2.0, 0.0],
            point![2.0, 0.0],
            &mut planner,
            &[
                PathSegment::LineSegment(LineSegment(point![-2.0, 0.0], point![-0.5, 0.866])),
                PathSegment::Arc(
                    Arc {
                        circle: Circle {
                            center: point![0.0, 0.0],
                            radius: 1.0,
                        },
                        start: point![-0.5, 0.866],
                        end: point![0.5, 0.866],
                    },
                    Orientation::Clockwise,
                ),
                PathSegment::LineSegment(LineSegment(point![0.5, 0.866], point![2.0, 0.0])),
            ],
            4.511,
        );
    }

    #[test]
    fn path_around_multiple_circles() {
        let mut planner = PathPlanner::default();
        planner.with_obstacles(
            &[
                Obstacle::goal_post(point![-1.0, 0.0], 0.7001),
                Obstacle::goal_post(point![1.0, 0.0], 0.7001),
                Obstacle::goal_post(point![0.0, 2.0], 0.8),
            ],
            0.3,
        );
        run_test_scenario(
            point![-1.4, 1.0],
            point![1.4, 1.0],
            &mut planner,
            &[
                PathSegment::LineSegment(LineSegment(
                    point![-1.4, 1.0],
                    point![-0.9474172, 0.9756069],
                )),
                PathSegment::Arc(
                    Arc {
                        circle: Circle {
                            center: point![-1.0, 0.0],
                            radius: 0.9770229,
                        },
                        start: point![-0.9474172, 0.9756069],
                        end: point![-0.91782254, 0.9735608],
                    },
                    Orientation::Clockwise,
                ),
                PathSegment::LineSegment(LineSegment(
                    point![-0.91782254, 0.9735608],
                    point![-0.092521094, 0.90389776],
                )),
                PathSegment::Arc(
                    Arc {
                        circle: Circle {
                            center: point![0.0, 2.0],
                            radius: 1.1,
                        },
                        start: point![-0.092521094, 0.90389776],
                        end: point![0.09252105, 0.90389776],
                    },
                    Orientation::Counterclockwise,
                ),
                PathSegment::LineSegment(LineSegment(
                    point![0.09252105, 0.90389776],
                    point![0.91782254, 0.9735608],
                )),
                PathSegment::Arc(
                    Arc {
                        circle: Circle {
                            center: point![1.0, 0.0],
                            radius: 0.9770229,
                        },
                        start: point![0.91782254, 0.9735608],
                        end: point![0.9474171, 0.97560686],
                    },
                    Orientation::Clockwise,
                ),
                PathSegment::LineSegment(LineSegment(
                    point![0.9474171, 0.97560686],
                    point![1.4, 1.0],
                )),
            ],
            2.8,
        );
    }

    #[test]
    fn path_around_ball() {
        let mut planner = PathPlanner::default();
        planner.with_obstacles(&[Obstacle::ball(point![-0.76, 0.56], 0.25)], 0.0);
        run_test_scenario(
            point![0.0, 0.0],
            point![-0.99, 0.66],
            &mut planner,
            &[
                PathSegment::LineSegment(LineSegment(
                    point![0.0, 0.0],
                    point![-0.8465765, 0.35145843],
                )),
                PathSegment::Arc(
                    Arc {
                        circle: Circle {
                            center: point![-0.76, 0.56],
                            radius: 0.22579876,
                        },
                        start: point![-0.8465765, 0.35145843],
                        end: point![-0.9856166, 0.55093247],
                    },
                    Orientation::Clockwise,
                ),
                PathSegment::LineSegment(LineSegment(
                    point![-0.9856166, 0.55093247],
                    point![-0.99, 0.66],
                )),
            ],
            1.28,
        );
    }

    #[test]
    fn path_ball_and_robot_near_goalpost() {
        let mut planner = PathPlanner::default();
        planner.with_obstacles(
            &[
                Obstacle::ball(point![2.454_799_4, -0.584_156_7], 0.05),
                Obstacle::goal_post(point![2.290_639_2, 0.022_267_818], 0.05),
                Obstacle::goal_post(point![0.798_598_23, 0.600_034], 0.05),
            ],
            0.3,
        );
        run_test_scenario(
            Point2::origin(),
            point![2.641_596_3, -0.247_508_54],
            &mut planner,
            &[
                PathSegment::LineSegment(LineSegment(
                    point![0.0, 0.0],
                    point![2.2338033, 0.3676223],
                )),
                PathSegment::Arc(
                    Arc {
                        circle: Circle {
                            center: point![2.2906392, 0.022267818],
                            radius: 0.35000002,
                        },
                        start: point![2.2338033, 0.3676223],
                        end: point![2.640637, 0.02350672],
                    },
                    Orientation::Clockwise,
                ),
                PathSegment::LineSegment(LineSegment(
                    point![2.640637, 0.02350672],
                    point![2.6415963, -0.24750854],
                )),
            ],
            PI,
        );
    }

    #[test]
    fn path_ball_near_goalpost() {
        let mut map = PathPlanner::default();
        map.with_obstacles(
            &[
                Obstacle::ball(point![3.925_943_6, 0.885_463_5], 0.05),
                Obstacle::goal_post(point![2.180_831, 1.564_113_6], 0.05),
                Obstacle::goal_post(point![3.780_714, 1.544_771_2], 0.05),
                Obstacle::goal_post(point![2.072_028_6, -7.435_228_3], 0.05),
                Obstacle::goal_post(point![3.671_911_7, -7.454_571], 0.05),
            ],
            0.3,
        );
        run_test_scenario(
            Point2::origin(),
            point![3.944_771_8, 1.034_277_4],
            &mut map,
            &[
                PathSegment::LineSegment(LineSegment(
                    point![0.0, 0.0],
                    point![3.8195379, 1.2188969],
                )),
                PathSegment::Arc(
                    Arc {
                        circle: Circle {
                            center: point![3.9259436, 0.8854635],
                            radius: 0.35,
                        },
                        start: point![3.8195379, 1.2188969],
                        end: point![3.8212261, 1.2194309],
                    },
                    Orientation::Clockwise,
                ),
                PathSegment::LineSegment(LineSegment(
                    point![3.8212261, 1.2194309],
                    point![3.9742692, 1.2674185],
                )),
            ],
            4.17,
        );
    }

    #[test]
    fn path_start_surrounded() {
        let mut map = PathPlanner::default();
        map.with_obstacles(
            &[
                Obstacle::goal_post(point![0.5, 0.5], 0.6),
                Obstacle::goal_post(point![-0.5, 0.5], 0.6),
                Obstacle::goal_post(point![-0.5, -0.5], 0.6),
                Obstacle::goal_post(point![0.5, -0.5], 0.6),
            ],
            0.0,
        );
        assert!(map
            .plan(Point2::origin(), point![2.0, 0.0])
            .expect("Path error")
            .is_none());
    }

    #[test]
    fn path_end_surrounded() {
        let mut map = PathPlanner::default();
        map.with_obstacles(
            &[
                Obstacle::goal_post(point![0.5, 0.5], 0.6),
                Obstacle::goal_post(point![-0.5, 0.5], 0.6),
                Obstacle::goal_post(point![-0.5, -0.5], 0.6),
                Obstacle::goal_post(point![0.5, -0.5], 0.6),
            ],
            0.0,
        );
        assert!(map
            .plan(point![2.0, 0.0], Point2::origin())
            .expect("Path error")
            .is_none());
    }
}
