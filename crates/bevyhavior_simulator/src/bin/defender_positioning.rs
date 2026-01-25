use bevy::prelude::*;

use hsl_network_messages::{GameState, PlayerNumber};
use linear_algebra::{point, vector};
use scenario::scenario;

use bevyhavior_simulator::{
    ball::BallResource,
    game_controller::GameControllerCommand,
    robot::Robot,
    time::{Ticks, TicksTime},
};
use types::ball_position::SimulatorBallState;

#[scenario]
fn defender_positioning(app: &mut App) {
    app.add_systems(Startup, startup);
    app.add_systems(Update, update);
}

fn startup(
    mut commands: Commands,
    mut game_controller_commands: MessageWriter<GameControllerCommand>,
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
    game_controller_commands.write(GameControllerCommand::SetGameState(GameState::Ready));
}

fn update(
    time: Res<Time<Ticks>>,
    mut ball: ResMut<BallResource>,
    mut exit: MessageWriter<AppExit>,
) {
    if time.ticks() == 5000 {
        ball.state = Some(SimulatorBallState {
            position: point![-3.6, 2.5],
            velocity: vector![0.0, 0.0],
        });
    }

    if time.ticks() == 8000 {
        ball.state = Some(SimulatorBallState {
            position: point![-3.6, -2.5],
            velocity: vector![0.0, 0.0],
        });
    }

    if time.ticks() == 16000 {
        ball.state = Some(SimulatorBallState {
            position: point![-1.5, 0.0],
            velocity: vector![-3.0, -0.2],
        });
    }

    if time.ticks() == 30000 {
        exit.write(AppExit::Success);
    }
}
