use bevy::prelude::*;

use linear_algebra::{point, vector, Isometry2, Vector2};
use scenario::scenario;
use spl_network_messages::{GameState, PlayerNumber};

use bevyhavior_simulator::{
    ball::BallResource,
    game_controller::{GameController, GameControllerCommand},
    robot::Robot,
    time::{Ticks, TicksTime},
};
use types::{ball_position::SimulatorBallState, roles::Role};

#[scenario]
fn striker_from_unseen_ball(app: &mut App) {
    app.add_systems(Startup, startup);
    app.add_systems(Update, update);
}

fn startup(
    mut commands: Commands,
    mut ball: ResMut<BallResource>,
    mut game_controller_commands: EventWriter<GameControllerCommand>,
) {
    let mut one = Robot::new(PlayerNumber::One);
    *one.ground_to_field_mut() = Isometry2::from_parts(vector![-2.0, -0.2], 0.0);
    commands.spawn(one);
    let mut two = Robot::new(PlayerNumber::Two);
    // 0.00001 is necessary to avoid #1038 for some reason
    *two.ground_to_field_mut() = Isometry2::from_parts(vector![0.0, 0.00001], 0.0);
    commands.spawn(two);

    ball.state = Some(SimulatorBallState {
        position: point![0.0, -0.3],
        velocity: Vector2::zeros(),
    });

    game_controller_commands.send(GameControllerCommand::SetGameState(GameState::Playing));
}

fn update(
    game_controller: ResMut<GameController>,
    time: Res<Time<Ticks>>,
    robots: Query<&Robot>,
    mut exit: EventWriter<AppExit>,
) {
    if time.ticks() == 40
        && !robots.iter().any(|robot| {
            robot.parameters.player_number == PlayerNumber::Two
                && robot.database.main_outputs.role == Role::Striker
        })
    {
        println!("Error: Two didn't become striker when sent a nearby ball position");
        exit.send(AppExit::from_code(2));
    }

    if game_controller.state.hulks_team.score > 0 {
        println!("Done");
        exit.send(AppExit::Success);
    }
    if time.ticks() >= 10_000 {
        println!("No goal was scored :(");
        exit.send(AppExit::from_code(1));
    }
}
