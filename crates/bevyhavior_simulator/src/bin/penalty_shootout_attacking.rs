use bevy::prelude::*;

use linear_algebra::{point, vector, Isometry2, Vector};
use scenario::scenario;
use spl_network_messages::{GameState, PlayerNumber, Team};
use types::ball_position::SimulatorBallState;

use bevyhavior_simulator::{
    ball::BallResource,
    game_controller::{GameController, GameControllerCommand},
    robot::Robot,
    time::{Ticks, TicksTime},
};

#[scenario]
fn penalty_shootout_attacking(app: &mut App) {
    app.add_systems(Startup, startup);
    app.add_systems(Update, update);
}

fn startup(
    mut commands: Commands,
    mut game_controller_commands: EventWriter<GameControllerCommand>,
    mut ball: ResMut<BallResource>,
) {
    let mut robot = Robot::new(PlayerNumber::One);
    *robot.ground_to_field_mut() = Isometry2::from_parts(vector![2.8, 0.0], 0.0);
    commands.spawn(robot);
    ball.state = Some(SimulatorBallState {
        position: point!(3.2, 0.0),
        velocity: Vector::zeros(),
    });
    game_controller_commands.send(GameControllerCommand::SetGamePhase(
        spl_network_messages::GamePhase::PenaltyShootout {
            kicking_team: Team::Hulks,
        },
    ));
    game_controller_commands.send(GameControllerCommand::SetGameState(GameState::Set));
}

#[allow(clippy::too_many_arguments)]
fn update(
    game_controller: ResMut<GameController>,
    time: ResMut<Time<Ticks>>,
    mut exit: EventWriter<AppExit>,
) {
    if game_controller.state.hulks_team.score > 0 {
        println!("Done");
        exit.send(AppExit::Success);
    }
    if time.ticks() >= 10_000 {
        println!("No goal was scored :(");
        exit.send(AppExit::from_code(1));
    }
}
