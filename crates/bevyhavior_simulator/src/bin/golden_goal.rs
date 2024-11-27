use bevy::prelude::*;

use scenario::scenario;
use spl_network_messages::GameState;

use bevyhavior_simulator::{
    hulks_setup::hulks_setup,
    game_controller::{GameController, GameControllerCommand},
    time::{Ticks, TicksTime},
};

#[scenario]
fn golden_goal(app: &mut App) {
    app.add_systems(Startup, startup);
    app.add_systems(Update, update);
}

fn startup(commands: Commands, mut game_controller_commands: EventWriter<GameControllerCommand>) {
    let active_field_players = vec![5, 7, 9, 13, 15, 17, 20];
    let picked_up_players = vec![];
    let goal_keeper_jersey_number = 17;
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
}
