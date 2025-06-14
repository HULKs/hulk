use std::f32::consts::FRAC_PI_2;

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

    game_controller_commands.send(GameControllerCommand::SetGameState(GameState::Playing));
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
    robots.single_mut().database.main_outputs.ground_to_field =
        Some(Isometry2::from_parts(vector![-1.0, -1.0], FRAC_PI_2));

    let optimizer_steps = time.ticks() as usize;
    robots
        .single_mut()
        .parameters
        .step_planner
        .optimization_parameters
        .optimizer_steps = optimizer_steps;

    println!("tick {}: {optimizer_steps} steps", time.ticks());

    if time.ticks() >= 500 {
        exit.send(AppExit::Success);
    }
}
