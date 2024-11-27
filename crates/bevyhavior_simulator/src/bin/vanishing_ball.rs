use bevy::prelude::*;

use scenario::scenario;
use spl_network_messages::GameState;

use bevyhavior_simulator::{
    hulks_setup::hulks_setup,
    ball::BallResource,
    game_controller::GameControllerCommand,
    time::{Ticks, TicksTime},
};

#[scenario]
fn vanishing_ball(app: &mut App) {
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
    mut ball: ResMut<BallResource>,
    mut exit: EventWriter<AppExit>,
    mut game_controller_commands: EventWriter<GameControllerCommand>,
    time: Res<Time<Ticks>>,
) {
    if time.ticks() == 2 {
        game_controller_commands.send(GameControllerCommand::SetGameState(GameState::Ready));
    }
    if time.ticks() == 2800 {
        ball.state = None;
    }
    if time.ticks() >= 10_000 {
        println!("Done");
        exit.send(AppExit::Success);
    }
}
