use bevy::prelude::*;

use linear_algebra::point;
use scenario::scenario;
use spl_network_messages::{GameState, SubState, Team};

use bevyhavior_simulator::{
    aufstellung::hulks_aufstellung,
    ball::BallResource,
    game_controller::{GameController, GameControllerCommand},
    time::{Ticks, TicksTime},
};
use types::ball_position::SimulatorBallState;

#[scenario]
fn ball_search(app: &mut App) {
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
    game_controller: ResMut<GameController>,
    mut ball: ResMut<BallResource>,
    mut exit: EventWriter<AppExit>,
    mut game_controller_commands: EventWriter<GameControllerCommand>,
    time: Res<Time<Ticks>>,
) {
    if time.ticks() == 2 {
        game_controller_commands.send(GameControllerCommand::SetGameState(GameState::Ready));
    }
    if time.ticks() == 2000 {
        game_controller_commands.send(GameControllerCommand::SetSubState(
            Some(SubState::CornerKick),
            Team::Opponent,
        ));
    }
    if time.ticks() == 5000 {
        ball.state = Some(SimulatorBallState {
            position: point![3.5, -2.0],
            velocity: Default::default(),
        });
    }
    if game_controller.state.hulks_team.score > 0 {
        println!("Done");
        exit.send(AppExit::Success);
    }
    if time.ticks() >= 15_000 {
        println!("No goal was scored :(");
        exit.send(AppExit::from_code(1));
    }
}
