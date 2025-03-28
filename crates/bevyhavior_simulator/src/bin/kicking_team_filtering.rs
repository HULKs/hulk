use std::time::SystemTime;

use bevy::{ecs::system::SystemParam, prelude::*};

use linear_algebra::vector;
use scenario::scenario;
use spl_network_messages::{GameState, PlayerNumber, SubState, Team};

use bevyhavior_simulator::{
    ball::BallResource,
    game_controller::GameControllerCommand,
    robot::Robot,
    time::{Ticks, TicksTime},
};
use types::{
    ball_position::{BallPosition, SimulatorBallState},
    filtered_game_controller_state::FilteredGameControllerState,
    filtered_game_state::FilteredGameState,
};

const TICKS_PER_FREE_KICK: u32 = 83 * 30;

/// Is used to generate the test functions for cargo test
#[scenario]
fn visual_referee_free_kick_behavior(app: &mut App) {
    app.add_systems(Startup, startup);
    app.add_systems(Update, update);
}

#[derive(SystemParam)]
struct State {}

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
    mut game_controller_commands: EventWriter<GameControllerCommand>,
    time: Res<Time<Ticks>>,
    mut exit: EventWriter<AppExit>,
    mut robots: Query<&mut Robot>,
    mut ball: ResMut<BallResource>,
) {
    if let Some(ball_state) = ball.state {
        if ball_state.position.x() >= 4.3 && ball_state.position.y().abs() < 1.5 {
            ball.state = Some(SimulatorBallState {
                position: ball_state.position,
                velocity: vector!(-3.0, 0.0),
            })
        }
    }

    if time.ticks() >= 3000 && time.ticks() < 3000 + TICKS_PER_FREE_KICK {
        check_kicking_team_inference(
            &time,
            3000,
            &mut game_controller_commands,
            &mut exit,
            &mut robots,
            &ball,
            SubState::CornerKick,
            Team::Hulks,
            true,
        );
    }

    if time.ticks() >= 6000 && time.ticks() < 6000 + TICKS_PER_FREE_KICK {
        check_kicking_team_inference(
            &time,
            6000,
            &mut game_controller_commands,
            &mut exit,
            &mut robots,
            &ball,
            SubState::GoalKick,
            Team::Opponent,
            false,
        );
    }

    if time.ticks() >= 9000 && time.ticks() < 9000 + 2 * TICKS_PER_FREE_KICK {
        check_kicking_team_inference(
            &time,
            9000,
            &mut game_controller_commands,
            &mut exit,
            &mut robots,
            &ball,
            SubState::PenaltyKick,
            Team::Hulks,
            true,
        );
    }

    if time.ticks() >= 15000 && time.ticks() < 15000 + TICKS_PER_FREE_KICK {
        check_kicking_team_inference(
            &time,
            15000,
            &mut game_controller_commands,
            &mut exit,
            &mut robots,
            &ball,
            SubState::GoalKick,
            Team::Hulks,
            true,
        );
    }

    if time.ticks() >= 18000 && time.ticks() < 18000 + TICKS_PER_FREE_KICK {
        check_kicking_team_inference(
            &time,
            18000,
            &mut game_controller_commands,
            &mut exit,
            &mut robots,
            &ball,
            SubState::CornerKick,
            Team::Opponent,
            false,
        );
    }

    if time.ticks() >= 21000 && time.ticks() < 21000 + 2 * TICKS_PER_FREE_KICK {
        check_kicking_team_inference(
            &time,
            21000,
            &mut game_controller_commands,
            &mut exit,
            &mut robots,
            &ball,
            SubState::PenaltyKick,
            Team::Opponent,
            false,
        );
    }

    if time.ticks() >= 24000 && time.ticks() < 24000 + TICKS_PER_FREE_KICK {
        check_kicking_team_inference(
            &time,
            24000,
            &mut game_controller_commands,
            &mut exit,
            &mut robots,
            &ball,
            SubState::PushingFreeKick,
            Team::Opponent,
            false,
        );
    }

    if time.ticks() >= 27000 && time.ticks() < 27000 + TICKS_PER_FREE_KICK {
        check_kicking_team_inference(
            &time,
            27000,
            &mut game_controller_commands,
            &mut exit,
            &mut robots,
            &ball,
            SubState::PushingFreeKick,
            Team::Hulks,
            true,
        );
    }

    if time.ticks() >= 30_000 {
        println!("Done! Successfully and correctly inferred kicking team in all passively inferrable sub states.");
        exit.send(AppExit::Success);
    }
}

#[allow(clippy::too_many_arguments)]
fn check_kicking_team_inference(
    time: &Res<Time<Ticks>>,
    tick_start: u32,
    game_controller_commands: &mut EventWriter<GameControllerCommand>,
    exit: &mut EventWriter<AppExit>,
    robots: &mut Query<&mut Robot>,
    ball: &ResMut<BallResource>,
    checked_sub_state: SubState,
    correct_kicking_team: Team,
    correct_ball_is_free: bool,
) {
    if time.ticks() == tick_start {
        game_controller_commands.send(GameControllerCommand::SetSubState(
            Some(checked_sub_state),
            correct_kicking_team,
        ));
    }
    if let Some(ball_state) = ball.state {
        for mut robot in &mut *robots {
            robot.database.main_outputs.ball_position = Some(BallPosition {
                position: robot.ground_to_field().inverse() * ball_state.position,
                velocity: vector!(0.0, 0.0),
                last_seen: SystemTime::UNIX_EPOCH + time.elapsed(),
            })
        }
    }
    for robot in robots {
        match robot.database.main_outputs.filtered_game_controller_state {
            Some(FilteredGameControllerState {
                game_state: FilteredGameState::Playing { ball_is_free, .. },
                kicking_team,
                sub_state,
                ..
            }) if sub_state == Some(checked_sub_state)
                && (ball_is_free != correct_ball_is_free
                    || kicking_team != Some(correct_kicking_team)) =>
            {
                println!("Scenario failed. kicking_team and/or ball_is_free was not correctly inferred during {:?} with kicking team {:?}.", sub_state, correct_kicking_team);
                dbg!(&robot.database.main_outputs.filtered_game_controller_state);
                dbg!(&robot.database.additional_outputs.last_ball_state);
                exit.send(AppExit::from_code(1));
                return;
            }
            _ => (),
        }
    }
}
