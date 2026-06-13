use bevy::prelude::*;

use hsl_network_messages::{GameState, PlayerNumber};
use scenario::scenario;

use bevyhavior_simulator::{
    game_controller::{GameController, GameControllerCommand},
    robot::Robot,
    time::{Ticks, TicksTime},
};

#[scenario]
fn flickering_ball(app: &mut App) {
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
        let mut robot = Robot::new(number);
        robot.simulator_parameters.ball_timeout_factor = 0.001;
        commands.spawn(robot);
    }
    game_controller_commands.write(GameControllerCommand::SetGameState(GameState::Ready));
}

fn update(
    game_controller: ResMut<GameController>,
    time: Res<Time<Ticks>>,
    mut exit: MessageWriter<AppExit>,
    mut robots: Query<&mut Robot>,
) {
    for mut robot in robots.iter_mut() {
        robot.simulator_parameters.ball_view_range =
            (time.elapsed().as_secs_f32() * 10.0).sin() * 1.5 + 1.5
    }
    if game_controller.state.hulks_team.score > 0 {
        println!("Done");
        exit.write(AppExit::Success);
    }
    if time.ticks() >= 20_000 {
        println!("No goal was scored :(");
        exit.write(AppExit::from_code(1));
    }
}
