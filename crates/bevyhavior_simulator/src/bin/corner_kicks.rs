use bevy::prelude::*;

use linear_algebra::{point, vector};
use scenario::scenario;
use spl_network_messages::{GameState, PlayerNumber, SubState, Team};

use bevyhavior_simulator::{
    ball::BallResource,
    game_controller::{GameController, GameControllerCommand},
    robot::Robot,
    time::{Ticks, TicksTime},
};
use types::ball_position::SimulatorBallState;

#[scenario]
fn corner_kicks(app: &mut App) {
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
    mut exit: EventWriter<AppExit>,
    mut ball: ResMut<BallResource>,
) {
    if time.ticks() == 3000 {
        game_controller_commands.send(GameControllerCommand::SetSubState(
            Some(SubState::CornerKick),
            Team::Hulks,
        ));
    }

    if time.ticks() == 4500 {
        ball.state = Some(SimulatorBallState {
            position: point!(-2.25, 1.0),
            velocity: vector![-6.0, 2.0],
        });
    }

    if time.ticks() == 5000 {
        game_controller_commands.send(GameControllerCommand::SetSubState(
            Some(SubState::CornerKick),
            Team::Opponent,
        ));
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
