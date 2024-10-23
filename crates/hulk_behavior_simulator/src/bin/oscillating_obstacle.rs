use bevy::prelude::*;

use linear_algebra::{point, vector, Isometry2, Point2};
use scenario::scenario;
use spl_network_messages::{GameState, PlayerNumber};
use types::{
    ball_position::SimulatorBallState,
    obstacles::{Obstacle, ObstacleKind},
};

use hulk_behavior_simulator::{
    ball::BallResource,
    game_controller::{GameController, GameControllerCommand},
    robot::Robot,
    time::{Ticks, TicksTime},
};

#[scenario]
fn oscillating_obstacle(app: &mut App) {
    app.add_systems(Startup, startup);
    app.add_systems(Update, update);
}
fn startup(
    mut commands: Commands,
    mut game_controller: ResMut<GameController>,
    mut game_controller_commands: EventWriter<GameControllerCommand>,
    mut ball: ResMut<BallResource>,
) {
    let mut robot = Robot::new(PlayerNumber::Seven);
    *robot.ground_to_field_mut() = Isometry2::from_parts(vector![-2.0, 0.0], 0.0);
    commands.spawn(robot);
    game_controller.state.game_state = GameState::Playing;
    game_controller_commands.send(GameControllerCommand::SetGameState(GameState::Playing));
    ball.state = Some(SimulatorBallState {
        position: Point2::origin(),
        velocity: vector![2.0, 0.1],
    });
}

#[allow(clippy::too_many_arguments)]
fn update(
    game_controller: ResMut<GameController>,
    time: ResMut<Time<Ticks>>,
    mut exit: EventWriter<AppExit>,
    mut robots: Query<&mut Robot>,
) {
    let mut robot = robots.single_mut();
    let field_to_ground = robot.ground_to_field().inverse();
    robot.database.main_outputs.obstacles = vec![Obstacle {
        kind: ObstacleKind::Unknown,
        position: field_to_ground
            * point![
                0.02 * (time.ticks() as f32 / 3.0).sin(),
                0.05 * (time.ticks() as f32 / 2.0).cos()
            ],
        radius_at_foot_height: 0.5,
        radius_at_hip_height: 0.5,
    }];

    if game_controller.state.hulks_team.score > 0 {
        println!("Done");
        exit.send(AppExit::Success);
    }
    if time.ticks() >= 10_000 {
        println!("No goal was scored :(");
        exit.send(AppExit::from_code(1));
    }
}
