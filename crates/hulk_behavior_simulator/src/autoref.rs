use std::f32::consts::FRAC_PI_2;

use bevy::{
    app::{App, Update},
    ecs::{
        event::{EventReader, EventWriter},
        schedule::IntoSystemConfigs,
        system::{Query, ResMut, Resource},
    },
    time::{Time, Timer, TimerMode},
};
use linear_algebra::{vector, Isometry2};
use spl_network_messages::{GameState, Penalty, Team};
use types::{
    ball_position::SimulatorBallState, motion_command::MotionCommand, planned_path::PathSegment,
};

use crate::{
    ball::BallResource,
    game_controller::{GameController, GameControllerCommand},
    robot::Robot,
    whistle::WhistleResource,
};

#[derive(Resource)]
pub struct AutorefState {
    robots_standing_still: Option<Timer>,
    pub goal_mode: GoalMode,
}

#[derive(Default, Debug)]
pub enum GoalMode {
    #[default]
    GoToReady,
    Ignore,
    ReturnBall,
}

impl Default for AutorefState {
    fn default() -> Self {
        Self {
            robots_standing_still: None,
            goal_mode: GoalMode::GoToReady,
        }
    }
}

pub fn autoref(
    mut state: ResMut<AutorefState>,
    mut ball: ResMut<BallResource>,
    mut referee_whistle: ResMut<WhistleResource>,
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
            referee_whistle.whistle(*time);
        }
        GameState::Playing => {
            if let Some(scoring_team) = ball.state.and_then(ball_in_goal) {
                match state.goal_mode {
                    GoalMode::GoToReady => {
                        game_controller_commands.send(GameControllerCommand::Goal(scoring_team));
                        ball.state = None;
                    }
                    GoalMode::ReturnBall => {
                        ball.state = Some(SimulatorBallState::default());
                    }
                    GoalMode::Ignore => {}
                }
            }
        }
        _ => {}
    }
}

fn ball_in_goal(ball: SimulatorBallState) -> Option<Team> {
    if ball.position.x() > 4.5 && ball.position.y().abs() < 0.75 {
        return Some(Team::Hulks);
    }
    if ball.position.x() < -4.5 && ball.position.y().abs() < 0.75 {
        return Some(Team::Opponent);
    }

    None
}

pub fn auto_assistant_referee(
    mut game_controller_commands: EventReader<GameControllerCommand>,
    mut robots: Query<&mut Robot>,
) {
    for command in game_controller_commands.read() {
        match *command {
            GameControllerCommand::SetGameState(_) => {}
            GameControllerCommand::SetKickingTeam(_) => {}
            GameControllerCommand::Goal(_) => {}
            GameControllerCommand::Penalize(player_number, penalty) => match penalty {
                Penalty::IllegalMotionInStandby { .. } | Penalty::IllegalMotionInSet { .. } => {
                    // Robots are penalized in place
                }
                Penalty::IllegalBallContact { .. }
                | Penalty::PlayerPushing { .. }
                | Penalty::InactivePlayer { .. }
                | Penalty::IllegalPosition { .. }
                | Penalty::LeavingTheField { .. }
                | Penalty::RequestForPickup { .. }
                | Penalty::LocalGameStuck { .. }
                | Penalty::IllegalPositionInSet { .. }
                | Penalty::PlayerStance { .. }
                | Penalty::Substitute { .. }
                | Penalty::Manual { .. } => {
                    if let Some(mut robot) = robots
                        .iter_mut()
                        .find(|robot| robot.parameters.player_number == player_number)
                    {
                        *robot.ground_to_field_mut() =
                            Isometry2::from_parts(vector![-3.2, -3.3], FRAC_PI_2);
                    }
                }
            },
            GameControllerCommand::Unpenalize(player_number) => {
                if let Some(mut robot) = robots
                    .iter_mut()
                    .find(|robot| robot.parameters.player_number == player_number)
                {
                    *robot.ground_to_field_mut() =
                        Isometry2::from_parts(vector![-3.2, -3.3], FRAC_PI_2);
                }
            }
        }
    }
}

pub fn autoref_plugin(app: &mut App) {
    app.add_systems(Update, autoref);
    app.add_systems(Update, auto_assistant_referee.after(autoref));
    app.init_resource::<AutorefState>();
}
