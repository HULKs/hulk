use std::sync::Arc;
use std::{boxed::Box, future::Future, pin::Pin};

use color_eyre::Result;
use geometry::{circle::Circle, rectangle::Rectangle};
use hsl_network_messages::{SubState, Team};
use linear_algebra::{Point, point, vector};
use ros_z::{prelude::*, qos::QosDurability};
use serde::{Deserialize, Serialize};
use types::{
    field_dimensions::FieldDimensions, filtered_game_controller_state::FilteredGameControllerState,
    filtered_game_state::FilteredGameState,
    rule_obstacles::RuleObstacle, time_wrapper::TimeWrapper, world_state::BallState,
};

#[derive(Debug, Clone, Serialize, Deserialize, Message)]
#[serde(deny_unknown_fields)]
pub struct Parameters {
    pub center_circle_obstacle_radius_increase: f32,
    pub center_circle_ballspace_free_obstacle_radius: f32,
    pub free_kick_obstacle_radius: f32,
    pub penaltykick_box_extension: f32,
}

pub fn run_boxed(ctx: Arc<Context>) -> Pin<Box<dyn Future<Output = Result<()>> + Send>> {
    Box::pin(run(ctx))
}

async fn run(ctx: Arc<Context>) -> Result<()> {
    let node = ctx.create_node("rule_obstacle_composer").build().await?;

    let parameters = node.bind_parameter_as::<Parameters>("rule_obstacle_composer")?;
    let field_dimensions_cache = node
        .subscriber::<FieldDimensions>("field_dimensions")
        .qos(QosProfile {
            durability: QosDurability::TransientLocal,
            ..Default::default()
        })
        .cache(1)
        .build()
        .await?;
    let filtered_game_controller_state_sub = node
        .subscriber::<FilteredGameControllerState>("filtered_game_controller_state")
        .build()
        .await?;
    let ball_state_cache = node
        .subscriber::<Option<BallState>>("ball_state")
        .cache(1)
        .build()
        .await?;
    let rule_obstacles_pub = node
        .publisher::<Vec<RuleObstacle>>("rule_obstacles")
        .build()
        .await?;

    loop {
        let filtered_game_controller_state = filtered_game_controller_state_sub.recv().await?;
        let Some(field_dimensions) = field_dimensions_cache.get_latest() else {
            continue;
        };
        let ball_state = ball_state_cache.get_latest().and_then(|ball| *ball);

        let parameters_snapshot = parameters.snapshot();
        let parameters = parameters_snapshot.typed();
        let rule_obstacles = compose_rule_obstacles(
            &filtered_game_controller_state,
            ball_state,
            field_dimensions.as_ref(),
            parameters,
        );

        rule_obstacles_pub.publish(&rule_obstacles).await?;
    }
}

fn compose_rule_obstacles(
    filtered_game_controller_state: &FilteredGameControllerState,
    ball_state: Option<BallState>,
    field_dimensions: &FieldDimensions,
    parameters: &Parameters,
) -> Vec<RuleObstacle> {
    let mut rule_obstacles = Vec::new();

    match (filtered_game_controller_state, ball_state) {
        (
            FilteredGameControllerState {
                sub_state:
                    Some(
                        SubState::ThrowIn
                        | SubState::CornerKick
                        | SubState::GoalKick
                        | SubState::DirectFreeKick
                        | SubState::IndirectFreeKick,
                    ),
                kicking_team: Some(Team::Opponent),
                game_state: FilteredGameState::Playing { .. },
                ..
            },
            Some(ball),
        ) => {
            let free_kick_obstacle = RuleObstacle::Circle(Circle::new(
                ball.ball_in_field,
                parameters.free_kick_obstacle_radius,
            ));
            rule_obstacles.push(free_kick_obstacle);
        }
        (
            FilteredGameControllerState {
                game_state:
                    FilteredGameState::Playing {
                        ball_is_free: false,
                        kick_off: true,
                    },
                ..
            },
            _,
        ) => {
            let center_circle_obstacle = RuleObstacle::Circle(Circle::new(
                Point::origin(),
                field_dimensions.center_circle_diameter / 2.0
                    + parameters.center_circle_obstacle_radius_increase,
            ));
            rule_obstacles.push(center_circle_obstacle);

            let opponent_half_obstacle = RuleObstacle::Rectangle(Rectangle {
                min: point!(0.0, -field_dimensions.width / 2.0),
                max: point!(field_dimensions.length / 2.0, field_dimensions.width / 2.0),
            });
            rule_obstacles.push(opponent_half_obstacle);
        }
        (
            FilteredGameControllerState {
                sub_state: Some(SubState::PenaltyKick),
                game_state: FilteredGameState::Playing { .. },
                ..
            },
            _,
        ) => match filtered_game_controller_state.kicking_team {
            Some(Team::Hulks) => rule_obstacles.push(create_penalty_box(
                field_dimensions,
                Team::Hulks,
                parameters.penaltykick_box_extension,
            )),
            Some(Team::Opponent) => rule_obstacles.push(create_penalty_box(
                field_dimensions,
                Team::Opponent,
                parameters.penaltykick_box_extension,
            )),
            None => {
                rule_obstacles.push(create_penalty_box(
                    field_dimensions,
                    Team::Hulks,
                    parameters.penaltykick_box_extension,
                ));
                rule_obstacles.push(create_penalty_box(
                    field_dimensions,
                    Team::Opponent,
                    parameters.penaltykick_box_extension,
                ));
            }
        },
        (
            FilteredGameControllerState {
                game_state: FilteredGameState::Ready,
                sub_state: None,
                kicking_team: Some(Team::Hulks),
                ..
            },
            _,
        ) => {
            let center_circle_ballspace_free_obstacle = RuleObstacle::Circle(Circle {
                center: Point::origin(),
                radius: parameters.center_circle_ballspace_free_obstacle_radius,
            });

            rule_obstacles.push(center_circle_ballspace_free_obstacle);
        }
        (
            FilteredGameControllerState {
                game_state: FilteredGameState::Ready,
                sub_state: None,
                kicking_team: None,
                ..
            }
            | FilteredGameControllerState {
                game_state: FilteredGameState::Ready,
                sub_state: None,
                kicking_team: Some(Team::Opponent),
                ..
            },
            _,
        ) => {
            let center_circle_obstacle = RuleObstacle::Circle(Circle {
                center: Point::origin(),
                radius: field_dimensions.center_circle_diameter / 2.0
                    + parameters.center_circle_obstacle_radius_increase,
            });

            rule_obstacles.push(center_circle_obstacle);
        }
        _ => (),
    };

    rule_obstacles
}

fn create_penalty_box(
    field_dimensions: &FieldDimensions,
    kicking_team: Team,
    penaltykick_box_extension: f32,
) -> RuleObstacle {
    let side_factor: f32 = match kicking_team {
        Team::Hulks => 1.0,
        Team::Opponent => -1.0,
    };

    let half_field_length = field_dimensions.length / 2.0;
    let half_penalty_area_length = field_dimensions.penalty_area_length / 2.0;
    let center_x = side_factor
        * (half_field_length - half_penalty_area_length + penaltykick_box_extension / 2.0);
    RuleObstacle::Rectangle(Rectangle::new_with_center_and_size(
        point![center_x, 0.0],
        vector![
            field_dimensions.penalty_area_length + penaltykick_box_extension,
            field_dimensions.penalty_area_width
        ],
    ))
}
