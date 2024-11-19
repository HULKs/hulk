use bevy::prelude::*;

use linear_algebra::point;
use scenario::scenario;
use spl_network_messages::{GameState, PlayerNumber};

use bevyhavior_simulator::{
    ball::BallResource,
    game_controller::GameControllerCommand,
    robot::Robot,
    time::{Ticks, TicksTime},
};

#[scenario]
fn walk_around_ball(app: &mut App) {
    app.add_systems(Startup, startup);
    app.add_systems(Update, update);
}

fn startup(
    mut commands: Commands,
    mut game_controller_commands: EventWriter<GameControllerCommand>,
) {
    commands.spawn(Robot::new(PlayerNumber::Seven));
    game_controller_commands.send(GameControllerCommand::SetGameState(GameState::Ready));
}

fn update(time: Res<Time<Ticks>>, mut ball: ResMut<BallResource>, mut exit: EventWriter<AppExit>) {
    if time.ticks() == 2_800 {
        if let Some(ball) = ball.state.as_mut() {
            ball.position = point![0.0, 0.0];
        }
    }
    if time.ticks() >= 10_000 {
        println!("Done");
        exit.send(AppExit::Success);
    }
}
