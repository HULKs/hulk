use std::time::Duration;

use bevy::prelude::*;

use scenario::scenario;
use spl_network_messages::{GameState, Penalty, PlayerNumber};

use bevyhavior_simulator::{
    game_controller::{GameController, GameControllerCommand},
    robot::Robot,
    time::{Ticks, TicksTime},
};
use types::{primary_state::PrimaryState, roles::Role};

#[scenario]
fn golden_goal(app: &mut App) {
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
    game_controller_commands.send(GameControllerCommand::Penalize(
        PlayerNumber::Seven,
        Penalty::Manual {
            remaining: Duration::from_secs(80),
        },
    ));
    game_controller_commands.send(GameControllerCommand::SetGameState(GameState::Ready));
}

fn update(
    game_controller: ResMut<GameController>,
    robots: Query<&Robot>,
    time: Res<Time<Ticks>>,
    mut exit: EventWriter<AppExit>,
) {
    if game_controller.state.hulks_team.score > 0 {
        println!("Done");
        exit.send(AppExit::Success);
    }
    if time.ticks() >= 10_000 {
        println!("No goal was scored :(");
        exit.send(AppExit::from_code(1));
    }

    let striker_count = robots
        .iter()
        .filter(|robot| robot.database.main_outputs.primary_state != PrimaryState::Penalized)
        .filter(|robot| robot.database.main_outputs.role == Role::Striker)
        .count();
    if game_controller.state.game_state == GameState::Set {
        if striker_count == 1 {
            println!("One striker is present");
            exit.send(AppExit::Success);
        } else {
            println!("Error: Found {striker_count} strikers!");
            exit.send(AppExit::from_code(1));
        }
    }
}
