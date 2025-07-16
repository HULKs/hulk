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

const FREE_KICK_DURATION_IN_TICKS: u32 = 83 * 30;
const PENALTY_DURATION_IN_TICKS: u32 = 83 * 45;

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

    for (tick, sub_state, team) in [
        (3000, SubState::CornerKick, Team::Hulks),
        (12_000, SubState::GoalKick, Team::Opponent),
        (18_000, SubState::PenaltyKick, Team::Hulks),
        (30_000, SubState::GoalKick, Team::Hulks),
        (36_000, SubState::CornerKick, Team::Opponent),
        (42_000, SubState::PenaltyKick, Team::Opponent),
        (52_000, SubState::PushingFreeKick, Team::Opponent),
        (60_000, SubState::PushingFreeKick, Team::Hulks),
    ] {
        set_substate_at_tick_start(
            &time,
            tick,
            &ball,
            &mut robots,
            &mut game_controller_commands,
            sub_state,
            team,
        );
        let check_duration = match sub_state {
            SubState::PenaltyKick => 2 * FREE_KICK_DURATION_IN_TICKS,
            _ => FREE_KICK_DURATION_IN_TICKS,
        };
        if (tick + 5..tick + check_duration).contains(&time.ticks()) {
            check_kicking_team_inference(&time, &mut exit, &robots, sub_state, team);
        }
    }

    if time.ticks() >= 28_000 {
        if let Some(ball_state) = ball.state {
            if ball_state.position.x() >= -0.5 {
                ball.state = Some(SimulatorBallState {
                    position: ball_state.position,
                    velocity: vector!(-3.0, 0.0),
                })
            }
        }
    }

    if time.ticks() == 18_000 + PENALTY_DURATION_IN_TICKS {
        game_controller_commands.send(GameControllerCommand::Unpenalize(
            PlayerNumber::Six,
            Team::Opponent,
        ));
    }

    if time.ticks() == 42_000 + PENALTY_DURATION_IN_TICKS {
        game_controller_commands.send(GameControllerCommand::Unpenalize(
            PlayerNumber::Six,
            Team::Hulks,
        ));
    }

    if time.ticks() == 52_000 + PENALTY_DURATION_IN_TICKS {
        game_controller_commands.send(GameControllerCommand::Unpenalize(
            PlayerNumber::Six,
            Team::Hulks,
        ));
    }

    if time.ticks() == 60_000 + PENALTY_DURATION_IN_TICKS {
        game_controller_commands.send(GameControllerCommand::Unpenalize(
            PlayerNumber::Six,
            Team::Opponent,
        ));
    }

    if time.ticks() >= 66_000 {
        println!("Done! Successfully and correctly inferred kicking team in all passively inferrable sub states.");
        exit.send(AppExit::Success);
    }
}

#[allow(clippy::too_many_arguments)]
fn check_kicking_team_inference(
    time: &Res<Time<Ticks>>,
    exit: &mut EventWriter<AppExit>,
    robots: &Query<&mut Robot>,
    checked_sub_state: SubState,
    correct_kicking_team: Team,
) {
    let correct_ball_is_free = correct_kicking_team == Team::Hulks;
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
                dbg!(sub_state);
                dbg!(ball_is_free);
                dbg!(kicking_team);
                println!("{} Scenario failed. kicking_team and/or ball_is_free was not correctly inferred during {:?} with kicking team {:?}. {}", time.ticks(), sub_state.unwrap(), correct_kicking_team, robot.parameters.player_number);
                exit.send(AppExit::from_code(1));
                return;
            }
            _ => (),
        }
    }
}

fn set_substate_at_tick_start(
    time: &Res<Time<Ticks>>,
    tick_start: u32,
    ball: &ResMut<BallResource>,
    robots: &mut Query<&mut Robot>,
    game_controller_commands: &mut EventWriter<GameControllerCommand>,
    checked_sub_state: SubState,
    correct_kicking_team: Team,
) {
    if let Some(ball_state) = ball.state {
        for mut robot in &mut *robots {
            robot.database.main_outputs.ball_position = Some(BallPosition {
                position: robot.ground_to_field().inverse() * ball_state.position,
                velocity: vector!(0.0, 0.0),
                last_seen: SystemTime::UNIX_EPOCH + time.elapsed(),
            })
        }
    }
    if time.ticks() == tick_start {
        let penalized_player_number =
            if [SubState::PenaltyKick, SubState::PushingFreeKick].contains(&checked_sub_state) {
                Some(PlayerNumber::Six)
            } else {
                None
            };

        game_controller_commands.send(GameControllerCommand::SetSubState(
            Some(checked_sub_state),
            correct_kicking_team,
            penalized_player_number,
        ));
    }
}
