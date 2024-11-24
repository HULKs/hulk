use std::time::Duration;

use bevy::prelude::*;

use scenario::scenario;
use spl_network_messages::GameState;

use bevyhavior_simulator::{
    game_controller::{GameController, GameControllerCommand},
    robot::Robot,
    time::{Ticks, TicksTime},
};

#[scenario]
fn golden_goal(app: &mut App) {
    app.add_systems(Startup, startup);
    app.add_systems(Update, update);
}

fn startup(
    mut commands: Commands,
    mut game_controller_commands: EventWriter<GameControllerCommand>,
) {
    for number in 8..=20 {
        game_controller_commands.send(GameControllerCommand::Penalize(
            number,
            spl_network_messages::Penalty::Substitute {
                remaining: Duration::MAX,
            },
        ));
    }
    for number in [1, 2, 3, 4, 5, 6, 7] {
        commands.spawn(Robot::new(number));
    }
    game_controller_commands.send(GameControllerCommand::SetGameState(GameState::Ready));
}

fn update(
    game_controller: ResMut<GameController>,
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
}
