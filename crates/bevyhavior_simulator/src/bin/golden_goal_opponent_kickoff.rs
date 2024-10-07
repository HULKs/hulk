use bevy::prelude::*;

use scenario::scenario;
use spl_network_messages::{GameState, Team};

use hulk_behavior_simulator::{
    aufstellung::hulks_aufstellung,
    game_controller::{GameController, GameControllerCommand},
    time::{Ticks, TicksTime},
};

#[scenario]
fn golden_goal_opponent_kickoff(app: &mut App) {
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
    game_controller_commands.send(GameControllerCommand::SetGameState(GameState::Ready));
    game_controller_commands.send(GameControllerCommand::SetKickingTeam(Team::Opponent));
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
