use bevy::prelude::*;

use linear_algebra::point;
use scenario::scenario;
use spl_network_messages::{GameState, PlayerNumber};

use hulk_behavior_simulator::{
    ball::BallResource,
    game_controller::{GameController, GameControllerCommand},
    robot::Robot,
    time::{Ticks, TicksTime},
};

#[scenario]
fn quantum_ball(app: &mut App) {
    app.add_systems(Startup, startup);
    app.add_systems(Update, update);
}
fn startup(
    mut commands: Commands,
    mut game_controller_commands: EventWriter<GameControllerCommand>,
) {
    for number in [
        PlayerNumber::One,
        PlayerNumber::Two,
        PlayerNumber::Three,
        PlayerNumber::Four,
        PlayerNumber::Five,
        PlayerNumber::Six,
        PlayerNumber::Seven,
    ] {
        commands.spawn(Robot::new(number));
    }
    game_controller_commands.send(GameControllerCommand::SetGameState(GameState::Ready));
}
fn update(
    game_controller: ResMut<GameController>,
    time: Res<Time<Ticks>>,
    mut exit: EventWriter<AppExit>,
    mut ball: ResMut<BallResource>,
) {
    if game_controller.state.hulks_team.score > 0 {
        println!("Done");
        exit.send(AppExit::Success);
    }
    if time.ticks() >= 10_000 {
        println!("No goal was scored :(");
        exit.send(AppExit::from_code(1));
    }
    if time.ticks() == 1800 {
        if let Some(ball) = ball.state.as_mut() {
            ball.position = point![0.6, 0.5];
        }
    }

    if time.ticks() == 1900 {
        if let Some(ball) = ball.state.as_mut() {
            ball.position = point![-1.77, -2.0];
        }
    }

    if time.ticks() == 2100 {
        if let Some(ball) = ball.state.as_mut() {
            ball.position = point![-1.8, 3.0];
        }
    }

    if time.ticks() == 2300 {
        if let Some(ball) = ball.state.as_mut() {
            ball.position = point![3.31, 0.0];
        }
    }

    if time.ticks() == 2600 {
        if let Some(ball) = ball.state.as_mut() {
            ball.position = point![1.31, -1.0];
        }
    }

    if time.ticks() == 2900 {
        if let Some(ball) = ball.state.as_mut() {
            ball.position = point![-2.01, 2.0];
        }
    }

    if time.ticks() == 3300 {
        if let Some(ball) = ball.state.as_mut() {
            ball.position = point![1.38, 3.0];
        }
    }

    if time.ticks() == 3800 {
        if let Some(ball) = ball.state.as_mut() {
            ball.position = point![-1.51, -2.0];
        }
    }
}
