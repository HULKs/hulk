use bevy::prelude::*;

use hsl_network_messages::{GameState, PlayerNumber, SubState, Team};
use scenario::scenario;

use bevyhavior_simulator::{
    game_controller::{GameController, GameControllerCommand},
    robot::Robot,
    time::{Ticks, TicksTime},
};

#[scenario]
fn goal_kicks(app: &mut App) {
    app.add_systems(Startup, startup);
    app.add_systems(Update, update);
}

fn startup(
    mut commands: Commands,
    mut game_controller_commands: MessageWriter<GameControllerCommand>,
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
    mut game_controller_commands: MessageWriter<GameControllerCommand>,
    time: Res<Time<Ticks>>,
    mut exit: MessageWriter<AppExit>,
) {
    if time.ticks() == 3000 {
        game_controller_commands.write(GameControllerCommand::SetSubState(
            Some(SubState::GoalKick),
            Team::Hulks,
            None,
        ));
    }
    if time.ticks() == 5000 {
        game_controller_commands.write(GameControllerCommand::SetSubState(
            Some(SubState::GoalKick),
            Team::Opponent,
            None,
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
