use bevy::prelude::*;

use bevyhavior_simulator::{
    ball::BallResource,
    game_controller::{GameController, GameControllerCommand},
    robot::Robot,
    time::{Ticks, TicksTime},
};
use linear_algebra::point;
use scenario::scenario;
use spl_network_messages::{GameState, PlayerNumber};
use types::roles::Role;

#[scenario]
fn reappearing_ball_in_front_of_keeper(app: &mut App) {
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
    time: Res<Time<Ticks>>,
    mut ball: ResMut<BallResource>,
    robots: Query<&Robot>,
    mut exit: EventWriter<AppExit>,
    mut keeper_was_striker_once: Local<bool>,
) {
    if time.ticks() == 2800 {
        if let Some(ball) = ball.state.as_mut() {
            ball.position = point![-3.8, 0.0];
        }
    }
    if game_controller.state.game_state == GameState::Playing {
        for robot in robots.iter() {
            if robot.parameters.player_number == PlayerNumber::One
                && robot.database.main_outputs.role == Role::Striker
            {
                *keeper_was_striker_once = true;
            }
        }
    }

    if game_controller.state.hulks_team.score > 0 {
        if !*keeper_was_striker_once {
            println!("Error: Keeper never became striker");
            exit.send(AppExit::from_code(2));
            return;
        }
        println!("Done");
        exit.send(AppExit::Success);
    }
    if time.ticks() >= 10_000 {
        println!("No goal was scored :(");
        exit.send(AppExit::from_code(1));
    }
}
