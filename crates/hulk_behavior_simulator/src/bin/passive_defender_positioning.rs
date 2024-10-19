use bevy::prelude::*;

use linear_algebra::{point, vector};
use scenario::scenario;
use spl_network_messages::{GameState, PlayerNumber};

use hulk_behavior_simulator::{
    ball::BallResource,
    game_controller::GameControllerCommand,
    robot::Robot,
    time::{Ticks, TicksTime},
};
use types::{ball_position::SimulatorBallState, motion_command::MotionCommand, roles::Role};

#[scenario]
fn passive_defender_positioning(app: &mut App) {
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
    time: Res<Time<Ticks>>,
    mut exit: EventWriter<AppExit>,
    mut ball: ResMut<BallResource>,
    mut robots: Query<&mut Robot>,
) {
    if time.ticks() == 4500 {
        ball.state = Some(SimulatorBallState {
            position: point!(2.25, 0.0),
            velocity: vector![-3.0, -1.0],
        });
    }
    if time.ticks() > 4500 && time.ticks() <= 4700 {
        for robot in robots.iter_mut() {
            match robot.database.main_outputs.role {
                Role::DefenderLeft | Role::DefenderRight => {
                    let motion_command = &robot.database.main_outputs.motion_command;
                    match motion_command {
                        MotionCommand::Stand { .. } => {}
                        _ => {
                            println!("Defenders moved unnecessarily");
                            exit.send(AppExit::from_code(1));
                        }
                    }
                }
                _ => {}
            }
        }
    }
    if time.ticks() >= 8_000 {
        println!("Done");
        exit.send(AppExit::Success);
    }
}
