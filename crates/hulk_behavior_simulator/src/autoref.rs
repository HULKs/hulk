use bevy::{
    app::{App, Update},
    ecs::system::{Query, ResMut, Resource},
    time::Time,
};
use spl_network_messages::GameState;
use types::ball_position::SimulatorBallState;

use crate::{ball::BallResource, game_controller::GameController, robot::Robot};

#[derive(Resource, Default)]
struct AutorefData {
    last_state_change: Time,
}

fn autoref(
    mut autoref: ResMut<AutorefData>,
    mut robots: Query<&mut Robot>,
    mut ball: ResMut<BallResource>,
    mut game_controller: ResMut<GameController>,
    time: ResMut<Time>,
) {
    match game_controller.state.game_state {
        GameState::Initial => {
            for mut robot in &mut robots {
                robot.is_penalized = false;
            }
            game_controller.state.game_state = GameState::Standby;
            autoref.last_state_change = time.as_generic();
        }
        GameState::Standby => {
            if (time.elapsed() - autoref.last_state_change.elapsed()).as_secs_f32() > 5.0 {
                game_controller.state.game_state = GameState::Set;
                autoref.last_state_change = time.as_generic();
            }
        }
        GameState::Ready => {
            if (time.elapsed() - autoref.last_state_change.elapsed()).as_secs_f32() > 30.0 {
                game_controller.state.game_state = GameState::Set;
                autoref.last_state_change = time.as_generic();
            }
        }
        GameState::Set => {
            if (time.elapsed() - autoref.last_state_change.elapsed()).as_secs_f32() > 3.0 {
                game_controller.state.game_state = GameState::Playing;
                ball.state = Some(SimulatorBallState::default());
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
