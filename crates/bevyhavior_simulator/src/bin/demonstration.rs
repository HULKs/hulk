use std::time::Duration;

use bevy::prelude::*;

use linear_algebra::{vector, Isometry2};
use scenario::scenario;
use spl_network_messages::{GameState, Penalty, SubState, Team};

use bevyhavior_simulator::{
    hulks_setup::hulks_setup,
    ball::BallResource,
    game_controller::{GameController, GameControllerCommand},
    robot::Robot,
    time::{Ticks, TicksTime},
};

/// Is used to generate the test functions for cargo test
#[scenario]
fn demonstration(app: &mut App) {
    app.add_systems(Startup, startup);
    app.add_systems(Update, update);
}
/// Runs at the start of the behavior simulator and is used to spawn in robots and set GameStates
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

/// Allows for checks to run during the scenario such that it can be decided whether the scenario passes or fails.
/// Not all of the parameters are always needed.
/// For example, golden_goal only checks to see if a goal was scored within 10000 frames.
/// * `ball` - allows manually changing the balls position and velocity
/// * `exit` - used to send exit conditions in the event the scenario passes or fails
/// * `game_controller_commands` - gives access to commands that are equivalent of pushing buttons on the game controller
/// * `game_controller` - gives access to the central GameController state
/// * `robots` - gives access to robots' internal database
/// * `time` - game time, useful with .ticks() to get frame count
fn update(
    game_controller: ResMut<GameController>,
    mut ball: ResMut<BallResource>,
    mut exit: EventWriter<AppExit>,
    mut game_controller_commands: EventWriter<GameControllerCommand>,
    mut robots: Query<&mut Robot>,
    time: Res<Time<Ticks>>,
) {
    if time.ticks() == 2 {
        game_controller_commands.send(GameControllerCommand::SetGameState(GameState::Ready));
    }
    // Scenarios can pass if a certain condition is met
    if game_controller.state.hulks_team.score > 0 {
        println!("Done");
        exit.send(AppExit::Success);
    }
    // Or fail based on a certain condition, such as if scoring takes too long
    if time.ticks() >= 10_000 {
        println!("No goal was scored :(");
        exit.send(AppExit::from_code(1));
    }
    // Based on time or other conditions you can modify the game state
    if time.ticks() == 3000 {
        // Set substate
        game_controller_commands.send(GameControllerCommand::SetSubState(
            Some(SubState::PushingFreeKick),
            Team::Opponent,
        ));
        // Penalize robot
        game_controller_commands.send(GameControllerCommand::Penalize(
            4,
            Penalty::PlayerPushing {
                remaining: Duration::from_secs(45),
            },
        ));
        // Manually move robot to some location on field
        robots
            .iter_mut()
            .find(|robot| robot.parameters.jersey_number == 7)
            .unwrap()
            .database
            .main_outputs
            .ground_to_field = Some(Isometry2::from_parts(vector![1.0, 1.0], 0.0));
        // Change the balls velocity
        if let Some(ball) = ball.state.as_mut() {
            ball.velocity = vector![0.0, -1.0];
        }
    }
}
