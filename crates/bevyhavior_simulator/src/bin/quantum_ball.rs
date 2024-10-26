use bevy::prelude::*;

use linear_algebra::point;
use scenario::scenario;
use spl_network_messages::GameState;

use bevyhavior_simulator::{
    aufstellung::hulks_aufstellung,
    ball::BallResource,
    game_controller::{GameController, GameControllerCommand},
    time::{Ticks, TicksTime},
};

#[scenario]
fn quantum_ball(app: &mut App) {
    app.add_systems(Startup, startup);
    app.add_systems(Update, update);
}
fn startup(commands: Commands, mut game_controller_commands: EventWriter<GameControllerCommand>) {
    let active_field_players = vec![1, 2, 3, 4, 5, 6, 7];
    let picked_up_players = vec![];
    let goal_keeper_jersey_number = 1;
    hulks_aufstellung(
        active_field_players,
        picked_up_players,
        goal_keeper_jersey_number,
        commands,
        &mut game_controller_commands,
    );
}
fn update(
    game_controller: ResMut<GameController>,
    mut ball: ResMut<BallResource>,
    mut exit: EventWriter<AppExit>,
    mut game_controller_commands: EventWriter<GameControllerCommand>,
    time: Res<Time<Ticks>>,
) {
    if time.ticks() == 2 {
        game_controller_commands.send(GameControllerCommand::SetGameState(GameState::Ready));
    }
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
