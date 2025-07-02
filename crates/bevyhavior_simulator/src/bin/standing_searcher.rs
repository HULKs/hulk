use bevy::prelude::*;

use linear_algebra::{point, vector, Isometry2};
use scenario::scenario;
use spl_network_messages::{GameState, PlayerNumber};

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
    game_controller_commands.send(GameControllerCommand::SetGameState(GameState::Ready));
}

fn update(
    time: Res<Time<Ticks>>,
    mut ball: ResMut<BallResource>,
    mut exit: EventWriter<AppExit>,
    mut robots: Query<&mut Robot>,
) {
    if time.ticks() == 4150 {
        robots
            .iter_mut()
            .find(|robot| robot.parameters.player_number == PlayerNumber::Two)
            .unwrap()
            .database
            .main_outputs
            .ground_to_field = Some(Isometry2::from_parts(vector![-3.0, 3.0], 0.0));
    }
    if time.ticks() == 4200 {
        ball.state = None;
    }
    if time.ticks() == 4400 {
        if let MotionCommand::Stand { head: _ } = robots
            .iter_mut()
            .find(|robot| robot.parameters.player_number == PlayerNumber::Two)
            .unwrap()
            .database
            .main_outputs
            .motion_command
        {
            println!("Standing searcher after penalty");
            exit.send(AppExit::from_code(1));
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
        exit.send(AppExit::Success);
    }
}
