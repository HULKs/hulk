use std::time::Duration;

use bevy::prelude::*;

use scenario::scenario;
use spl_network_messages::{GameState, Penalty};

use hulk_behavior_simulator::{
    autoref::{AutorefState, GoalMode},
    game_controller::GameControllerCommand,
    robot::Robot,
    time::{Ticks, TicksTime},
};
use types::roles::Role;

#[scenario]
fn replacement_keeper_test(app: &mut App) {
    app.add_systems(Startup, startup);
    app.add_systems(Update, update);
}

fn startup(
    mut commands: Commands,
    mut game_controller_commands: EventWriter<GameControllerCommand>,
    mut autoref: ResMut<AutorefState>,
) {
    for number in [1, 2, 3, 4, 5, 6, 7] {
        commands.spawn(Robot::new(number));
    }
    game_controller_commands.send(GameControllerCommand::SetGameState(GameState::Ready));
    autoref.goal_mode = GoalMode::ReturnBall;
}

fn update(
    mut game_controller_commands: EventWriter<GameControllerCommand>,
    mut robots: Query<&mut Robot>,
    time: Res<Time<Ticks>>,
    mut exit: EventWriter<AppExit>,
) {
    let replacement_keeper_count = robots
        .iter_mut()
        .filter(|robot| robot.database.main_outputs.role == Role::ReplacementKeeper)
        .count();

    if time.ticks() == 3000 || time.ticks() == 6000 {
        if replacement_keeper_count > 0 {
            println!("Unexpected replacement keeper");
            exit.send(AppExit::from_code(1));
        }
        game_controller_commands.send(GameControllerCommand::Penalize(
            1,
            Penalty::Manual {
                remaining: Duration::from_secs(5),
            },
        ));
    }

    if time.ticks() == 4000 || time.ticks() == 7000 {
        if replacement_keeper_count == 0 {
            println!("No robot became replacement keeper");
            exit.send(AppExit::from_code(1));
        }
        game_controller_commands.send(GameControllerCommand::Unpenalize(1));
    }

    if time.ticks() >= 10_000 {
        println!("Done");
        exit.send(AppExit::Success);
    }
}
