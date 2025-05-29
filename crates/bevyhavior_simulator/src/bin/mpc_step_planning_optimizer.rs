use bevy::prelude::*;

use bevyhavior_simulator::{
    game_controller::{GameController, GameControllerCommand},
    robot::Robot,
    time::{Ticks, TicksTime},
};
use linear_algebra::{vector, Isometry2};
use scenario::scenario;
use spl_network_messages::{GameState, PlayerNumber};

#[scenario]
fn mpc_step_planner_optimizer(app: &mut App) {
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

fn update(
    game_controller: ResMut<GameController>,
    time: Res<Time<Ticks>>,
    mut exit: EventWriter<AppExit>,
    mut robots: Query<&mut Robot>,
) {
    if game_controller.state.hulks_team.score > 0 {
        println!("Done");
        exit.send(AppExit::Success);
    }
    robots.single_mut().database.main_outputs.ground_to_field = Some(Isometry2::from_parts(
        vector![-0.612_85, -1.191_053_4],
        1.359_945_5,
    ));
    robots
        .single_mut()
        .parameters
        .step_planner
        .optimization_parameters
        .optimizer_steps = time.ticks() as usize / 20;
    println!("{}", time.ticks());
    if time.ticks() >= 3000 {
        exit.send(AppExit::Success);
    }
    if time.ticks() >= 10_000 {
        println!("No goal was scored :(");
        exit.send(AppExit::from_code(1));
    }
}
