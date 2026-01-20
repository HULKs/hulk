use std::time::Duration;

use bevy::prelude::*;

use hsl_network_messages::{GameState, Penalty, PlayerNumber, Team};
use linear_algebra::{point, vector};
use scenario::scenario;

use bevyhavior_simulator::{
    ball::BallResource,
    game_controller::GameControllerCommand,
    robot::Robot,
    time::{Ticks, TicksTime},
};
use types::{ball_position::SimulatorBallState, motion_command::MotionCommand};

#[scenario]
fn standing_searcher(app: &mut App) {
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
        PlayerNumber::Seven,
    ] {
        commands.spawn(Robot::new(number));
    }
    game_controller_commands.write(GameControllerCommand::SetGameState(GameState::Ready));
}

fn update(
    time: Res<Time<Ticks>>,
    mut ball: ResMut<BallResource>,
    mut exit: EventWriter<AppExit>,
    mut robots: Query<&mut Robot>,
    mut game_controller_commands: EventWriter<GameControllerCommand>,
) {
    if time.ticks() == 4150 {
        game_controller_commands.write(GameControllerCommand::Penalize(
            PlayerNumber::Two,
            Penalty::Manual {
                remaining: Duration::from_secs(1),
            },
            Team::Hulks,
        ));
    }

    if time.ticks() == 4155 {
        game_controller_commands.write(GameControllerCommand::Unpenalize(
            PlayerNumber::Two,
            Team::Hulks,
        ));
    }

    if time.ticks() == 4200 {
        ball.state = None;
    }

    if time.ticks() == 4660 {
        if let MotionCommand::Stand { .. } = robots
            .iter_mut()
            .find(|robot| robot.parameters.player_number == PlayerNumber::Two)
            .unwrap()
            .database
            .main_outputs
            .motion_command
        {
            println!("Standing searcher at penalty walk-in");
            exit.write(AppExit::from_code(1));
        }
        if let MotionCommand::Walk { .. } = robots
            .iter_mut()
            .find(|robot| robot.parameters.player_number == PlayerNumber::Three)
            .unwrap()
            .database
            .main_outputs
            .motion_command
        {
            println!("Moving searcher after ball loss");
            exit.write(AppExit::from_code(1));
        }
    }

    if time.ticks() == 5500 {
        ball.state = Some(SimulatorBallState {
            position: point![-2.7, -0.2],
            velocity: vector![0.0, 0.0],
        });
    }
    if time.ticks() == 6000 {
        ball.state = None;
    }
    if time.ticks() == 6750 {
        ball.state = Some(SimulatorBallState {
            position: point![4.0, -0.2],
            velocity: vector![0.0, 0.0],
        });
    }
    if time.ticks() == 7200 {
        ball.state = None;
    }
    if time.ticks() >= 10_000 {
        println!("Done");
        exit.write(AppExit::Success);
    }
}
