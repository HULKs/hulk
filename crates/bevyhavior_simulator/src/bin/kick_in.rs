use bevy::prelude::*;

use scenario::scenario;
use spl_network_messages::{GameState, PlayerNumber, SubState, Team};

use bevyhavior_simulator::{
    game_controller::{GameController, GameControllerCommand},
    robot::Robot,
    time::{Ticks, TicksTime},
};

const FREE_KICK_DURATION_IN_TICKS: u32 = 83 * 30;

#[scenario]
fn kick_in(app: &mut App) {
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
    mut game_controller_commands: EventWriter<GameControllerCommand>,
    time: Res<Time<Ticks>>,
    robots: Query<&mut Robot>,
    mut exit: EventWriter<AppExit>,
) {
    if time.ticks() == 5000 {
        game_controller_commands.send(GameControllerCommand::SetSubState(
            Some(SubState::KickIn),
            Team::Hulks,
            None,
        ));
    }
    if time.ticks() >= 5005 && time.ticks() < 5000 + FREE_KICK_DURATION_IN_TICKS {
        for robot in robots.iter() {
            if robot
                .database
                .main_outputs
                .filtered_game_controller_state
                .as_ref()
                .is_some_and(|state| state.kicking_team != Some(Team::Hulks))
            {
                println!(
                    "Robot {} did not correctly detect kicking team during kick in.",
                    robot.parameters.player_number
                );
                exit.send(AppExit::from_code(1));
            }
        }
    }

    if time.ticks() == 12000 {
        game_controller_commands.send(GameControllerCommand::SetSubState(
            Some(SubState::KickIn),
            Team::Opponent,
            None,
        ));
    }
    if time.ticks() >= 12005 && time.ticks() < 12000 + FREE_KICK_DURATION_IN_TICKS {
        for robot in robots.iter() {
            if robot
                .database
                .main_outputs
                .filtered_game_controller_state
                .as_ref()
                .is_some_and(|state| state.kicking_team != Some(Team::Opponent))
            {
                println!(
                    "Robot {} did not correctly detect kicking team during kick in.",
                    robot.parameters.player_number
                );
                exit.send(AppExit::from_code(1));
            }
        }
    }

    if game_controller.state.hulks_team.score > 0 {
        println!(
            "Done. Successfully detected the kicking team during kick ins and then scored a goal."
        );
        exit.send(AppExit::Success);
    }
    if time.ticks() >= 24_000 {
        println!("No goal was scored :(");
        exit.send(AppExit::from_code(1));
    }
}
