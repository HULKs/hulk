use bevy::prelude::*;

use linear_algebra::{point, vector};
use scenario::scenario;
use spl_network_messages::GameState;

use bevyhavior_simulator::{
    aufstellung::hulks_aufstellung,
    ball::BallResource,
    game_controller::GameControllerCommand,
    time::{Ticks, TicksTime},
};

#[scenario]
fn defender_positioning(app: &mut App) {
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
    if time.ticks() == 2500 {
        let state = ball.state.as_mut().expect("ball state not found");
        state.position = point![-3.6, 2.5];
    }

    if time.ticks() == 4000 {
        let state = ball.state.as_mut().expect("ball state not found");
        state.position = point![-3.6, -2.5];
    }

    if time.ticks() == 8000 {
        let state = ball.state.as_mut().expect("ball state not found");
        state.position = point![-1.5, 0.0];
        state.velocity = vector![-3.0, -0.2];
    }

    if time.ticks() == 15000 {
        exit.send(AppExit::Success);
    }
}
