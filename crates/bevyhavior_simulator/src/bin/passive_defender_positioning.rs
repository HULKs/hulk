use bevy::prelude::*;

use linear_algebra::{point, vector};
use scenario::scenario;
use spl_network_messages::GameState;

use bevyhavior_simulator::{
    hulks_setup::hulks_setup,
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
    mut robots: Query<&mut Robot>,
    time: ResMut<Time<Ticks>>,
) {
    if time.ticks() == 2 {
        game_controller_commands.send(GameControllerCommand::SetGameState(GameState::Ready));
    }
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
