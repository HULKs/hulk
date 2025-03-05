use bevy::prelude::*;

use bevyhavior_simulator::{
    ball::BallResource,
    game_controller::{GameController, GameControllerCommand},
    robot::Robot,
    time::{Ticks, TicksTime},
};
use linear_algebra::{point, Vector2};
use scenario::scenario;
use spl_network_messages::{GameState, PlayerNumber};
use types::roles::Role;

/// Regression test against an offensive keeper staying loser and never returning to the goal when losing the ball.
/// We lead the keeper away from the goal, then put the ball in front of the goal again.
/// If implemented correctly, the keeper should switch from loser to keeper after a short amount of
/// time.
#[scenario]
fn keeper_never_loser(app: &mut App) {
    app.add_systems(Startup, startup);
    app.add_systems(Update, update);
}

fn startup(
    mut commands: Commands,
    mut game_controller_commands: EventWriter<GameControllerCommand>,
) {
    for number in [PlayerNumber::One, PlayerNumber::Seven] {
        commands.spawn(Robot::new(number));
    }
    game_controller_commands.send(GameControllerCommand::SetGameState(GameState::Ready));
}

fn update(
    game_controller: ResMut<GameController>,
    time: Res<Time<Ticks>>,
    mut ball: ResMut<BallResource>,
    robots: Query<&Robot>,
    mut exit: EventWriter<AppExit>,
    mut keeper_was_striker_again: Local<bool>,
) {
    if time.ticks() == 2800 {
        if let Some(ball) = ball.state.as_mut() {
            ball.position = point![-3.8, 0.0];
            ball.velocity = Vector2::zeros();
        }
    }
    if time.ticks() == 6000 {
        if let Some(ball) = ball.state.as_mut() {
            ball.position = point![-3.8, 0.0];
            ball.velocity = Vector2::zeros();
        }
    }

    if time.ticks() > 6500 {
        for robot in robots.iter() {
            if robot.parameters.player_number == PlayerNumber::One
                && robot.database.main_outputs.role == Role::Striker
            {
                *keeper_was_striker_again = true;
            }
        }
    }

    if game_controller.state.hulks_team.score > 0 {
        println!("Done");
        exit.send(AppExit::Success);
    }

    if time.ticks() >= 15_000 {
        if !*keeper_was_striker_again {
            println!("Error: Keeper did not become striker again");
        }
        println!("No goal was scored :(");
        exit.send(AppExit::from_code(1));
    }
}
