use bevy::{
    app::{App, Update},
    ecs::{
        event::EventWriter,
        system::{Query, ResMut, Resource},
    },
    time::{Time, Timer, TimerMode},
};
use spl_network_messages::{GameState, Team};
use types::{
    ball_position::SimulatorBallState, motion_command::MotionCommand, planned_path::PathSegment,
};

use crate::{
    ball::BallResource,
    game_controller::{GameController, GameControllerCommand},
    robot::Robot,
};

#[derive(Resource, Default)]
struct AutorefState {
    robots_standing_still: Option<Timer>,
}

fn autoref(
    mut state: ResMut<AutorefState>,
    mut ball: ResMut<BallResource>,
    game_controller: ResMut<GameController>,
    mut game_controller_commands: EventWriter<GameControllerCommand>,
    robots: Query<&Robot>,
    time: ResMut<Time>,
) {
    match game_controller.state.game_state {
        GameState::Ready => {
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
            if let Some(timer) = state.robots_standing_still.as_mut() {
                timer.tick(time.delta());
            }
            if robots_moved_this_cycle {
                state.robots_standing_still = Some(Timer::from_seconds(1.0, TimerMode::Once));
            }

            if state
                .robots_standing_still
                .as_ref()
                .is_some_and(|timer| timer.finished())
            {
                game_controller_commands.send(GameControllerCommand::SetGameState(GameState::Set));
            }
        }
        GameState::Set => {
            if ball.state.is_none() {
                ball.state = Some(SimulatorBallState::default());
            };
        }
        GameState::Playing => {
            if let Some(ball_position) = ball.state.map(|ball_position| ball_position.position) {
                if ball_position.x() > 4.5 && ball_position.y().abs() < 0.75 {
                    ball.state = None;
                    game_controller_commands.send(GameControllerCommand::Goal(Team::Hulks));
                }
                if ball_position.x() < -4.5 && ball_position.y().abs() < 0.75 {
                    ball.state = None;
                    game_controller_commands.send(GameControllerCommand::Goal(Team::Opponent));
                }
            }
        }
        _ => {}
    }
}

pub fn autoref_plugin(app: &mut App) {
    app.add_systems(Update, autoref);
    app.init_resource::<AutorefState>();
}
