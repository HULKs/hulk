use bevy::prelude::*;

use linear_algebra::{vector, Isometry2};
use scenario::scenario;
use hsl_network_messages::{GameState, PlayerNumber, SubState, Team};

use bevyhavior_simulator::{
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
    game_controller_commands.write(GameControllerCommand::SetGameState(GameState::Ready));
}

/// Allows for checks to run during the scenario such that it can be decided whether the scenario passes or fails.
/// Not all of the parameters are always needed.
/// For example, golden_goal only checks to see if a goal was scored within 10000 frames.
/// * `game_controller` - gives access to the central GameController state
/// * `game_controller_commands` - gives access to commands that are equivalent of pushing buttons on the game controller
/// * `time` - game time, useful with .ticks() to get frame count
/// * `exit` - used to send exit conditions in the event the scenario passes or fails
/// * `robots` - gives access to robots' internal database
/// * `ball` - allows manually changing the balls position and velocity
fn update(
    game_controller: ResMut<GameController>,
    mut game_controller_commands: EventWriter<GameControllerCommand>,
    time: Res<Time<Ticks>>,
    mut exit: EventWriter<AppExit>,
    mut robots: Query<&mut Robot>,
    mut ball: ResMut<BallResource>,
) {
    // Scenarios can pass if a certain condition is met
    if game_controller.state.hulks_team.score > 0 {
        println!("Done");
        exit.write(AppExit::Success);
    }
    // Or fail based on a certain condition, such as if scoring takes too long
    if time.ticks() >= 20_000 {
        println!("No goal was scored :(");
        exit.write(AppExit::from_code(1));
    }
    // Based on time or other conditions you can modify the game state
    if time.ticks() == 6000 {
        // Set substate
        game_controller_commands.write(GameControllerCommand::SetSubState(
            Some(SubState::PushingFreeKick),
            Team::Opponent,
            Some(PlayerNumber::Four),
        ));
        // Manually move robot to some location on field
        robots
            .iter_mut()
            .find(|robot| robot.parameters.player_number == PlayerNumber::Seven)
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
