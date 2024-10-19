use bevy::prelude::*;

use linear_algebra::vector;
use scenario::scenario;
use spl_network_messages::{GameState, PlayerNumber};

use hulk_behavior_simulator::{
    ball::BallResource,
    game_controller::{GameController, GameControllerCommand},
    robot::Robot,
    time::{Ticks, TicksTime},
};

#[scenario]
fn hulks_vs_ghosts(app: &mut App) {
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
            ball.velocity = vector![-1.0, -1.0];
        }
    }

    if time.ticks() == 2200 {
        if let Some(ball) = ball.state.as_mut() {
            ball.velocity = vector![-2.0, 3.0];
        }
    }

    if time.ticks() == 2500 {
        if let Some(ball) = ball.state.as_mut() {
            ball.velocity = vector![-4.0, 0.0];
        }
    }

    if time.ticks() == 3000 {
        if let Some(ball) = ball.state.as_mut() {
            ball.velocity = vector![0.0, -1.0];
        }
    }

    if time.ticks() == 4000 {
        if let Some(ball) = ball.state.as_mut() {
            ball.velocity = vector![-3.5, -2.0];
        }
    }

    if time.ticks() == 4500 {
        if let Some(ball) = ball.state.as_mut() {
            ball.velocity = vector![0.0, 2.0];
        }
    }
}
