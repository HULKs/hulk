use std::time::Duration;

use bevy::prelude::*;

use scenario::scenario;
use spl_network_messages::{GameState, Penalty, PlayerNumber, SubState, Team};

use bevyhavior_simulator::{
    game_controller::{GameController, GameControllerCommand},
    robot::Robot,
    time::{Ticks, TicksTime},
};

#[scenario]
fn ingame_penalty_kick(app: &mut App) {
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
    game_controller_commands.write(GameControllerCommand::SetGameState(GameState::Ready));
}

fn update(
    game_controller: ResMut<GameController>,
    mut game_controller_commands: EventWriter<GameControllerCommand>,
    time: Res<Time<Ticks>>,
    mut exit: EventWriter<AppExit>,
) {
    if time.ticks() == 3000 {
        game_controller_commands.write(GameControllerCommand::SetSubState(
            Some(SubState::PenaltyKick),
            Team::Hulks,
            Some(PlayerNumber::Three),
        ));
    }
    if time.ticks() == 3050 {
        game_controller_commands.write(GameControllerCommand::Penalize(
            PlayerNumber::Seven,
            Penalty::Manual {
                remaining: Duration::from_secs(10),
            },
            Team::Hulks,
        ));
    }
    if time.ticks() == 3100 {
        game_controller_commands.write(GameControllerCommand::Penalize(
            PlayerNumber::Six,
            Penalty::Manual {
                remaining: Duration::from_secs(10),
            },
            Team::Hulks,
        ));
    }
    if time.ticks() == 3150 {
        game_controller_commands.write(GameControllerCommand::Penalize(
            PlayerNumber::Five,
            Penalty::Manual {
                remaining: Duration::from_secs(10),
            },
            Team::Hulks,
        ));
    }
    if game_controller.state.hulks_team.score > 0 {
        println!("Done");
        exit.write(AppExit::Success);
    }
    if time.ticks() >= 10_000 {
        println!("No goal was scored :(");
        exit.write(AppExit::from_code(1));
    }
}
