use bevy::{ecs::system::SystemParam, prelude::*};

use approx::AbsDiffEq;
use linear_algebra::point;
use scenario::scenario;
use spl_network_messages::{GameState, PlayerNumber, SubState, Team};

use bevyhavior_simulator::{
    ball::BallResource,
    game_controller::{GameController, GameControllerCommand},
    robot::Robot,
    time::{Ticks, TicksTime},
};
use types::{motion_command::HeadMotion, roles::Role};

/// Is used to generate the test functions for cargo test
#[scenario]
fn visual_referee_free_kick_behavior(app: &mut App) {
    app.add_systems(Startup, startup);
    app.add_systems(Update, update);
}

#[derive(SystemParam)]
struct State<'s> {
    detecting_robots_when_home: Local<'s, usize>,
    detecting_robots_when_away: Local<'s, usize>,
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
    game_controller_commands.send(GameControllerCommand::SetGameState(GameState::Ready));
}

fn update(
    mut game_controller: ResMut<GameController>,
    mut game_controller_commands: EventWriter<GameControllerCommand>,
    time: Res<Time<Ticks>>,
    mut exit: EventWriter<AppExit>,
    robots: Query<&mut Robot>,
    mut ball: ResMut<BallResource>,
    mut state: State,
) {
    if time.ticks() >= 10_000 {
        println!("Scenario failed: Time ran out. Behavior for detecting free kick kicking team was not executed correctly.");
        exit.send(AppExit::from_code(1));
    }

    if time.ticks() == 3000 {
        // Set substate
        game_controller_commands.send(GameControllerCommand::SetSubState(
            Some(SubState::PushingFreeKick),
            Team::Opponent,
        ));
    }

    if time.ticks() == 3005 {
        for relevant_robots in robots.iter().filter(|robot| {
            matches!(
                robot.database.main_outputs.role,
                Role::DefenderRight | Role::MidfielderRight
            )
        }) {
            if let Some(expected_referee_position) = relevant_robots
                .database
                .main_outputs
                .expected_referee_position
            {
                let ground_to_field = relevant_robots.ground_to_field();
                let expected_referee_position_in_ground =
                    ground_to_field.inverse() * expected_referee_position;
                if matches!(
                    relevant_robots.database.main_outputs.motion_command.head_motion(),
                    Some(HeadMotion::LookAt { target, .. }) if target.abs_diff_eq(&expected_referee_position_in_ground, 1e-4)
                ) {
                    *state.detecting_robots_when_home += 1;
                }
            }
        }
    }

    if time.ticks() == 3500 {
        // Set substate
        game_controller_commands.send(GameControllerCommand::SetSubState(None, Team::Opponent));
    }

    if time.ticks() == 4000 {
        // Set substate
        game_controller_commands.send(GameControllerCommand::SetSubState(
            Some(SubState::KickIn),
            Team::Opponent,
        ));

        game_controller.state.hulks_team_is_home_after_coin_toss = false;

        if let Some(ball) = ball.state.as_mut() {
            ball.position = point!(2.0, 4.5);
        }
    }
    if time.ticks() == 4002 {
        for relevant_robot in robots.iter().filter(|robot| {
            matches!(
                robot.database.main_outputs.role,
                Role::DefenderLeft | Role::MidfielderLeft
            )
        }) {
            if let Some(expected_referee_position) = relevant_robot
                .database
                .main_outputs
                .expected_referee_position
            {
                let ground_to_field = relevant_robot.ground_to_field();
                let expected_referee_position_in_ground =
                    ground_to_field.inverse() * expected_referee_position;

                if matches!(
                    relevant_robot.database.main_outputs.motion_command.head_motion(),
                    Some(HeadMotion::LookAt { target, .. }) if target.abs_diff_eq(&expected_referee_position_in_ground, 1e-4)
                ) {
                    *state.detecting_robots_when_away += 1;
                }
            }
        }
    }

    if (*state.detecting_robots_when_home == 2) && (*state.detecting_robots_when_away == 2) {
        println!("Done! Successfully performed behavior for free kick kicking team detection.");
        exit.send(AppExit::Success);
    }
}
