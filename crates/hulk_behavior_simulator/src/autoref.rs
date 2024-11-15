use std::f32::consts::FRAC_PI_2;

use bevy::{
    app::{App, Update},
    ecs::{
        event::{EventReader, EventWriter},
        schedule::IntoSystemConfigs,
        system::{Query, ResMut, Resource},
    },
    prelude::Res,
    time::{Time, Timer, TimerMode},
};
use linear_algebra::{point, vector, Isometry2};
use spl_network_messages::{GameState, Penalty, SubState, Team};
use types::{
    ball_position::SimulatorBallState,
    field_dimensions::{FieldDimensions, Half, Side},
    motion_command::MotionCommand,
    planned_path::PathSegment,
};

use crate::{
    ball::BallResource,
    field_dimensions::SimulatorFieldDimenstions,
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

#[allow(clippy::too_many_arguments)]
pub fn autoref(
    mut state: ResMut<AutorefState>,
    mut ball: ResMut<BallResource>,
    field_dimensions: Res<SimulatorFieldDimenstions>,
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
            if let Some(scoring_team) = ball
                .state
                .and_then(|ball| ball_in_goal(ball, **field_dimensions))
            {
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

fn ball_in_goal(ball: SimulatorBallState, field_dimensions: FieldDimensions) -> Option<Team> {
    if field_dimensions.is_inside_any_goal(ball.position) {
        if ball.position.x() > 0.0 {
            return Some(Team::Hulks);
        } else {
            return Some(Team::Opponent);
        }
    }
    None
}

pub fn auto_assistant_referee(
    mut game_controller_commands: EventReader<GameControllerCommand>,
    field_dimensions: Res<SimulatorFieldDimenstions>,
    mut robots: Query<&mut Robot>,
    mut ball: ResMut<BallResource>,
) {
    for command in game_controller_commands.read() {
        match *command {
            GameControllerCommand::SetGameState(_) => {}
            GameControllerCommand::SetGamePhase(_) => {}
            GameControllerCommand::SetSubState(Some(SubState::CornerKick), team) => {
                let side = if let Some(ball) = ball.state.as_mut() {
                    if ball.position.x() >= 0.0 {
                        Side::Left
                    } else {
                        Side::Right
                    }
                } else {
                    Side::Right
                };
                let half = match team {
                    Team::Hulks => Half::Opponent,
                    Team::Opponent => Half::Own,
                };
                ball.state = Some(SimulatorBallState {
                    position: field_dimensions.corner(half, side),
                    velocity: vector![0.0, 0.0],
                });
            }
            GameControllerCommand::SetSubState(Some(SubState::PenaltyKick), team) => {
                let half = match team {
                    Team::Hulks => Half::Opponent,
                    Team::Opponent => Half::Own,
                };
                ball.state = Some(SimulatorBallState {
                    position: field_dimensions.penalty_spot(half),
                    velocity: vector![0.0, 0.0],
                });
            }
            GameControllerCommand::SetSubState(Some(SubState::GoalKick), team) => {
                let side = if let Some(ball) = ball.state.as_mut() {
                    if ball.position.x() >= 0.0 {
                        Side::Left
                    } else {
                        Side::Right
                    }
                } else {
                    Side::Left
                };
                let half = match team {
                    Team::Hulks => Half::Own,
                    Team::Opponent => Half::Opponent,
                };
                ball.state = Some(SimulatorBallState {
                    position: field_dimensions.goal_box_corner(half, side),
                    velocity: vector![0.0, 0.0],
                });
            }
            GameControllerCommand::SetSubState(Some(SubState::KickIn), _) => {
                let x = if let Some(ball) = ball.state.as_mut() {
                    ball.position.x()
                } else {
                    0.0
                };
                let side = if let Some(ball) = ball.state.as_mut() {
                    if ball.position.x() >= 0.0 {
                        field_dimensions.width / 2.0
                    } else {
                        -field_dimensions.width / 2.0
                    }
                } else {
                    field_dimensions.width / 2.0
                };
                ball.state = Some(SimulatorBallState {
                    position: point!(x, side),
                    velocity: vector![0.0, 0.0],
                });
            }
            GameControllerCommand::SetSubState(..) => {}
            GameControllerCommand::BallIsFree => {}
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
