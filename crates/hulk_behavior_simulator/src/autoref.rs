use bevy::{
    app::{App, Update},
    ecs::system::{Query, ResMut, Resource},
    time::{Time, Timer, TimerMode},
};
use spl_network_messages::GameState;
use types::{
    ball_position::SimulatorBallState, motion_command::MotionCommand, planned_path::PathSegment,
};

use crate::{ball::BallResource, game_controller::GameController, robot::Robot};

#[derive(Resource, Default)]
struct AutorefData {
    last_state_change: Time,
    robots_standing_still: Timer,
    ball_spawn_timer: Timer,
}

fn autoref(
    mut autoref: ResMut<AutorefData>,
    mut ball: ResMut<BallResource>,
    mut game_controller: ResMut<GameController>,
    robots: Query<&Robot>,
    time: ResMut<Time>,
) {
    match game_controller.state.game_state {
        GameState::Initial => {
            game_controller.state.game_state = GameState::Standby;
            autoref.last_state_change = time.as_generic();
        }
        GameState::Standby => {
            if (time.elapsed() - autoref.last_state_change.elapsed()).as_secs_f32() > 5.0 {
                game_controller.state.game_state = GameState::Ready;
                autoref.last_state_change = time.as_generic();
            }
        }
        GameState::Ready => {
            let read_phase_time_out =
                (time.elapsed() - autoref.last_state_change.elapsed()).as_secs_f32() > 30.0;
            let robots_moved_this_cycle = robots.iter().any(|robot| -> bool {
                match &robot.database.main_outputs.motion_command {
                    MotionCommand::Unstiff
                    | MotionCommand::Penalized
                    | MotionCommand::Stand { .. } => false,
                    MotionCommand::Walk { path, .. }
                        if path.iter().map(PathSegment::length).sum::<f32>() < 0.01 =>
                    {
                        false
                    }
                    _ => true,
                }
            });
            autoref.robots_standing_still.tick(time.delta());
            if robots_moved_this_cycle {
                autoref.robots_standing_still = Timer::from_seconds(1.0, TimerMode::Once);
            }

            if read_phase_time_out || autoref.robots_standing_still.finished() {
                game_controller.state.game_state = GameState::Set;
                autoref.last_state_change = time.as_generic();
                autoref.ball_spawn_timer = Timer::from_seconds(1.5, TimerMode::Once)
            }
        }
        GameState::Set => {
            if autoref.ball_spawn_timer.tick(time.delta()).just_finished() {
                ball.state = Some(SimulatorBallState::default());
            };
            if (time.elapsed() - autoref.last_state_change.elapsed()).as_secs_f32() > 3.0 {
                game_controller.state.game_state = GameState::Playing;
                autoref.last_state_change = time.as_generic();
            }
        }
        GameState::Playing => {
            if let Some(ball_position) = ball.state.map(|ball_position| ball_position.position) {
                if ball_position.x() > 4.5 && ball_position.y().abs() < 0.75 {
                    ball.state = None;
                    game_controller.state.hulks_team.score += 1;
                    game_controller.state.game_state = GameState::Ready;
                    autoref.last_state_change = time.as_generic();
                }
                if ball_position.x() < -4.5 && ball_position.y().abs() < 0.75 {
                    ball.state = None;
                    game_controller.state.opponent_team.score += 1;
                    game_controller.state.game_state = GameState::Ready;
                    autoref.last_state_change = time.as_generic();
                }
            }
        }
        GameState::Finished => {}
    }
}

pub fn autoref_plugin(app: &mut App) {
    app.add_systems(Update, autoref);
    app.init_resource::<AutorefData>();
}
