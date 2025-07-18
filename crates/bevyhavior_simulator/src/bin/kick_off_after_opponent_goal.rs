use bevy::prelude::*;

use scenario::scenario;
use spl_network_messages::{GameState, PlayerNumber, Team};

use bevyhavior_simulator::{
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
    game_controller_commands.send(GameControllerCommand::SetGameState(GameState::Standby));
    game_controller_commands.send(GameControllerCommand::SetKickingTeam(Team::Opponent));
}
fn update(
    game_controller: ResMut<GameController>,
    time: Res<Time<Ticks>>,
    mut exit: EventWriter<AppExit>,
    mut game_controller_commands: EventWriter<GameControllerCommand>,
) {
    if time.ticks() == 100 {
        game_controller_commands.send(GameControllerCommand::SetGameState(GameState::Ready));
    }

    if game_controller.state.hulks_team.score > 0 {
        game_controller_commands.send(GameControllerCommand::SetGameState(GameState::Ready));
        game_controller_commands.send(GameControllerCommand::SetKickingTeam(Team::Opponent));
    }

    if time.ticks() == 10_000 {
        println!("Done");
        exit.send(AppExit::Success);
    }
}
