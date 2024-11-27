use bevy::prelude::*;

use linear_algebra::vector;
use scenario::scenario;
use spl_network_messages::GameState;

use bevyhavior_simulator::{
    hulks_setup::hulks_setup,
    ball::BallResource,
    game_controller::{GameController, GameControllerCommand},
    time::{Ticks, TicksTime},
};

#[scenario]
fn hulks_vs_ghosts(app: &mut App) {
    app.add_systems(Startup, startup);
    app.add_systems(Update, update);
}
fn startup(commands: Commands, mut game_controller_commands: EventWriter<GameControllerCommand>) {
    let active_field_players = vec![1, 2, 3, 4, 5, 6, 7];
    let picked_up_players = vec![];
    let goal_keeper_jersey_number = 1;
    hulks_setup(
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
