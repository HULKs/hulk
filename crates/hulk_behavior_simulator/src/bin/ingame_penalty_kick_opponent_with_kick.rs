use bevy::prelude::*;

use linear_algebra::vector;
use scenario::scenario;
use spl_network_messages::{GameState, PlayerNumber, SubState, Team};

use hulk_behavior_simulator::{
    ball::BallResource,
    game_controller::{GameController, GameControllerCommand},
    robot::Robot,
    time::{Ticks, TicksTime},
};

#[scenario]
fn ingame_penalty_kick_opponent_with_kick(app: &mut App) {
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
    mut ball: ResMut<BallResource>,
    time: Res<Time<Ticks>>,
    mut exit: EventWriter<AppExit>,
) {
    if time.ticks() == 3000 {
        game_controller_commands.send(GameControllerCommand::SetSubState(
            Some(SubState::PenaltyKick),
            Team::Opponent,
        ));
    }
    if time.ticks() == 6000 {
        game_controller_commands.send(GameControllerCommand::BallisFree);
        if let Some(ball) = ball.state.as_mut() {
            ball.velocity = vector![-2.0, 0.0];
        }
    }
    if game_controller.state.opponent_team.score > 0 {
        println!("Failed to prevent opponents from scoring");
        exit.send(AppExit::from_code(1));
    }
    if time.ticks() >= 10_000 {
        println!("Done");
        exit.send(AppExit::Success);
    }
}
