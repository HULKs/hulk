use std::f32::consts::FRAC_PI_2;

use bevy::{
    app::{App, Update},
    ecs::{
        event::{EventReader, EventWriter},
        resource::Resource,
        schedule::IntoScheduleConfigs,
        system::{Query, ResMut},
    },
    prelude::Res,
    time::{Time, Timer, TimerMode},
};
use coordinate_systems::{Field, Ground};
use linear_algebra::{point, vector, Isometry2};
use spl_network_messages::{GameState, Penalty, SubState, Team};
use step_planning::traits::Length;
use types::{
    ball_position::SimulatorBallState,
    field_dimensions::{FieldDimensions, Half, Side},
    motion_command::MotionCommand,
};

use crate::{
    ball::BallResource,
    field_dimensions::SimulatorFieldDimensions,
    game_controller::{GameController, GameControllerCommand},
    robot::Robot,
    visual_referee::VisualRefereeResource,
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
    field_dimensions: Res<SimulatorFieldDimensions>,
    mut referee_whistle: ResMut<WhistleResource>,
    game_controller: ResMut<GameController>,
    mut game_controller_commands: EventWriter<GameControllerCommand>,
    robots: Query<&Robot>,
    time: ResMut<Time>,
    mut visual_referee: ResMut<VisualRefereeResource>,
) {
    match game_controller.state.game_state {
        GameState::Ready => {
            let robots_moved_this_cycle = robots.iter().any(|robot| -> bool {
                match &robot.database.main_outputs.motion_command {
                    MotionCommand::Unstiff
                    | MotionCommand::Penalized
                    | MotionCommand::Stand { .. } => false,
                    MotionCommand::Walk { path, .. } if path.length() < 0.01 => false,
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
                game_controller_commands.write(GameControllerCommand::SetGameState(GameState::Set));
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
                        game_controller_commands.write(GameControllerCommand::Goal(scoring_team));
                        ball.state = None;
                    }
                    GoalMode::ReturnBall => {
                        ball.state = Some(SimulatorBallState::default());
                    }
                    GoalMode::Ignore => {}
                }
            }

            if game_controller.state.sub_state.is_some() {
                visual_referee.update_visual_referee(*time);
            } else {
                visual_referee.reset();
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
    field_dimensions: Res<SimulatorFieldDimensions>,
    mut robots: Query<&mut Robot>,
    mut ball: ResMut<BallResource>,
    game_controller: ResMut<GameController>,
    time: Res<Time>,
    mut visual_referee: ResMut<VisualRefereeResource>,
) {
    let penalized_walk_in_position: Isometry2<Ground, Field> =
        Isometry2::from_parts(vector![-3.2, -3.3], FRAC_PI_2);

    for command in game_controller_commands.read() {
        match *command {
            GameControllerCommand::SetGameState(_) => {}
            GameControllerCommand::SetGamePhase(_) => {}
            GameControllerCommand::SetSubState(sub_state, kicking_team, _) => {
                if sub_state.is_some() {
                    visual_referee.start_free_kick_pose(
                        *time,
                        kicking_team,
                        game_controller.state.global_field_side,
                    )
                };

                match sub_state {
                    Some(SubState::CornerKick) => {
                        let side = if let Some(ball) = ball.state {
                            if ball.position.y() >= 0.0 {
                                Side::Left
                            } else {
                                Side::Right
                            }
                        } else {
                            Side::Right
                        };
                        let half = match kicking_team {
                            Team::Hulks => Half::Opponent,
                            Team::Opponent => Half::Own,
                        };
                        ball.state = Some(SimulatorBallState {
                            position: field_dimensions.corner(half, side),
                            velocity: vector![0.0, 0.0],
                        });
                    }
                    Some(SubState::PenaltyKick) => {
                        let half = match kicking_team {
                            Team::Hulks => Half::Opponent,
                            Team::Opponent => Half::Own,
                        };
                        ball.state = Some(SimulatorBallState {
                            position: field_dimensions.penalty_spot(half),
                            velocity: vector![0.0, 0.0],
                        });
                    }
                    Some(SubState::GoalKick) => {
                        let side = if let Some(ball) = ball.state {
                            if ball.position.y() >= 0.0 {
                                Side::Left
                            } else {
                                Side::Right
                            }
                        } else {
                            Side::Left
                        };
                        let half = match kicking_team {
                            Team::Hulks => Half::Own,
                            Team::Opponent => Half::Opponent,
                        };
                        ball.state = Some(SimulatorBallState {
                            position: field_dimensions.goal_box_corner(half, side),
                            velocity: vector![0.0, 0.0],
                        });
                    }
                    Some(SubState::KickIn) => {
                        let position = if let Some(ball) = ball.state {
                            let x = ball.position.x();
                            let y = if ball.position.y() >= 0.0 {
                                field_dimensions.width / 2.0
                            } else {
                                -field_dimensions.width / 2.0
                            };
                            point![x, y]
                        } else {
                            point![0.0, field_dimensions.width / 2.0]
                        };
                        ball.state = Some(SimulatorBallState {
                            position,
                            velocity: vector![0.0, 0.0],
                        });
                    }
                    Some(SubState::PushingFreeKick) | None => {}
                }
            }
            GameControllerCommand::BallIsFree => {}
            GameControllerCommand::SetKickingTeam(_) => {}
            GameControllerCommand::Goal(_) => {}
            GameControllerCommand::Penalize(player_number, penalty, team) => match penalty {
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
                    if team == Team::Hulks {
                        if let Some(mut robot) = robots
                            .iter_mut()
                            .find(|robot| robot.parameters.player_number == player_number)
                        {
                            *robot.ground_to_field_mut() = penalized_walk_in_position;
                        }
                    }
                }
            },
            GameControllerCommand::Unpenalize(player_number, team) => {
                if team == Team::Hulks {
                    if let Some(mut robot) = robots
                        .iter_mut()
                        .find(|robot| robot.parameters.player_number == player_number)
                    {
                        *robot.ground_to_field_mut() = penalized_walk_in_position;
                    }
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
