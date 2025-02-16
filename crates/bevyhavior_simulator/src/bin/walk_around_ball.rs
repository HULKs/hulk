use std::time::SystemTime;

use bevy::prelude::*;

use linear_algebra::{point, Vector2};
use scenario::scenario;
use spl_network_messages::{GameState, PlayerNumber};

use bevyhavior_simulator::{
    ball::BallResource,
    game_controller::{GameController, GameControllerCommand},
    robot::Robot,
    time::{Ticks, TicksTime},
};
use types::ball_position::BallPosition;

#[scenario]
fn walk_around_ball(app: &mut App) {
    app.add_systems(Startup, startup);
    app.add_systems(Update, update);
}

fn startup(
    mut commands: Commands,
    mut game_controller_commands: EventWriter<GameControllerCommand>,
) {
    commands.spawn(Robot::new(PlayerNumber::Seven));
    game_controller_commands.send(GameControllerCommand::SetGameState(GameState::Ready));
}

fn update(
    game_controller: ResMut<GameController>,
    time: Res<Time<Ticks>>,
    mut robots: Query<&mut Robot>,
    mut ball: ResMut<BallResource>,
    mut exit: EventWriter<AppExit>,
) {
    if time.ticks() == 3200 {
        if let Some(ball) = ball.state.as_mut() {
            ball.position = point![0.0, 0.0];

            // loser never looks behind itself, so we need to tell it where the ball is
            let mut robot = robots.get_single_mut().unwrap();
            robot.database.main_outputs.ball_position = Some(BallPosition {
                position: robot.ground_to_field().inverse() * ball.position,
                velocity: Vector2::zeros(),
                last_seen: SystemTime::UNIX_EPOCH + time.elapsed(),
            });
        }
    }
    if game_controller.state.hulks_team.score > 0 {
        println!("Done");
        exit.send(AppExit::Success);
    }
    if time.ticks() >= 10_000 {
        println!("No goal was scored :(");
        exit.send(AppExit::from_code(1));
    }
}
