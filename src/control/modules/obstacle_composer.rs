use itertools::{chain, iproduct};
use module_derive::module;
use nalgebra::{point, Isometry2};
use types::{BallPosition, FieldDimensions, Obstacle, RobotPosition};

pub struct ObstacleComposer;

#[module(control)]
#[parameter(path = field_dimensions, data_type = FieldDimensions)]
#[input(path = ball_position, data_type = BallPosition)]
#[input(path = robot_positions, data_type = Vec<RobotPosition>)]
#[input(path = robot_to_field, data_type = Isometry2<f32>)]
#[main_output(data_type = Vec<Obstacle>, name = obstacles)]
impl ObstacleComposer {}

impl ObstacleComposer {
    fn new(_context: NewContext) -> anyhow::Result<Self> {
        Ok(Self)
    }

    fn cycle(&mut self, context: CycleContext) -> anyhow::Result<MainOutputs> {
        let field_dimensions = context.field_dimensions;
        let ball_position = context.ball_position;

        let ball_obstacle = ball_position.map(|ball_position| {
            Obstacle::ball(ball_position.position, field_dimensions.ball_radius)
        });

        let robot_positions = context.robot_positions;
        let robot_obstacles = match robot_positions {
            Some(robot_positions) => robot_positions
                .iter()
                .map(|obstacle_position| {
                    Obstacle::robot(obstacle_position.position, field_dimensions.ball_radius)
                })
                .collect::<Vec<_>>(),
            None => vec![],
        };

        let goal_post_obstacles = context
            .robot_to_field
            .map(|robot_to_field| {
                let field_to_robot = robot_to_field.inverse();
                iproduct!([-1.0, 1.0], [-1.0, 1.0]).map(move |(x_sign, y_sign)| {
                    let radius = field_dimensions.goal_post_diameter / 2.0;
                    let position_on_field = point![
                        x_sign * (field_dimensions.length / 2.0),
                        y_sign * (field_dimensions.goal_inner_width / 2.0 + radius)
                    ];
                    Obstacle::goal_post(field_to_robot * position_on_field, radius)
                })
            })
            .into_iter()
            .flatten();

        let obstacles = chain!(ball_obstacle, goal_post_obstacles, robot_obstacles).collect();

        Ok(MainOutputs {
            obstacles: Some(obstacles),
        })
    }
}
