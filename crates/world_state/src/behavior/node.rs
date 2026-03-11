use std::time::{SystemTime, UNIX_EPOCH};

use color_eyre::Result;
use hsl_network_messages::{SubState, Team};
use linear_algebra::{Point2, point};
use serde::{Deserialize, Serialize};

use context_attribute::context;
use coordinate_systems::{Field, Ground};
use framework::{AdditionalOutput, MainOutput};
use types::{
    action::Action,
    ball_position::BallPosition,
    cycle_time::CycleTime,
    field_dimensions::{FieldDimensions, Side},
    filtered_game_controller_state::FilteredGameControllerState,
    filtered_game_state::FilteredGameState,
    kick_decision::DecisionParameters,
    motion_command::MotionCommand,
    parameters::{BehaviorParameters, WalkSpeedParameters},
    path_obstacles::PathObstacle,
    primary_state::PrimaryState,
    roles::Role,
    world_state::WorldState,
};

use crate::behavior::{
    kicking, lost_ball, search, support, visual_kick, walk_to_kick_off, walk_to_penalty_kick,
};

use super::{
    defend::core::{Defend, DefendMode},
    finish,
    head::LookAction,
    initial, look_around, penalize, remote_control, safe, stand_during_penalty_kick, stand_up,
    stop, walk_to_ball,
    walk_to_pose::{WalkAndStand, WalkPathPlanner},
};

#[derive(Deserialize, Serialize)]
pub struct Behavior {
    last_defender_mode: DefendMode,
    active_since: Option<SystemTime>,
    last_known_ball_position: Point2<Field>,
    previous_role: Role,
    last_time_role_changed: SystemTime,
}

#[context]
pub struct CreationContext {}

#[context]
pub struct CycleContext {
    ball_position: Input<Option<BallPosition<Ground>>, "ball_position?">,
    cycle_time: Input<CycleTime, "cycle_time">,
    world_state: Input<WorldState, "world_state">,

    field_dimensions: Parameter<FieldDimensions, "field_dimensions">,
    kick_decision_parameters: Parameter<DecisionParameters, "kick_selector">,
    parameters: Parameter<BehaviorParameters, "behavior">,
    walk_speed: Parameter<WalkSpeedParameters, "walk_speed">,

    path_obstacles_output: AdditionalOutput<Vec<PathObstacle>, "path_obstacles">,
    active_action: AdditionalOutput<Action, "active_action">,

    last_motion_command: CyclerState<MotionCommand, "last_motion_command">,
}

#[context]
#[derive(Default)]
pub struct MainOutputs {
    pub motion_command: MainOutput<MotionCommand>,
}

impl Behavior {
    pub fn new(_context: CreationContext) -> Result<Self> {
        Ok(Self {
            last_defender_mode: DefendMode::Passive,
            active_since: None,
            last_known_ball_position: point![0.0, 0.0],
            previous_role: Role::Searcher,
            last_time_role_changed: UNIX_EPOCH,
        })
    }

    pub fn cycle(&mut self, mut context: CycleContext) -> Result<MainOutputs> {
        let world_state = context.world_state;

        if let Some(command) = &context.parameters.injected_motion_command {
            return Ok(MainOutputs {
                motion_command: command.clone().into(),
            });
        }

        let now = context.cycle_time.start_time;
        match (self.active_since, world_state.robot.primary_state) {
            (None, PrimaryState::Ready | PrimaryState::Set | PrimaryState::Playing) => {
                self.active_since = Some(now)
            }
            (None, _) => {}
            (Some(_), PrimaryState::Ready | PrimaryState::Set | PrimaryState::Playing) => {}
            (Some(_), _) => self.active_since = None,
        }
        if let Some(ball_state) = &world_state.ball {
            self.last_known_ball_position = ball_state.ball_in_field;
        }

        let mut actions = vec![
            Action::Safe,
            Action::Stop,
            Action::Finish,
            Action::Penalize,
            Action::Initial,
            Action::StandUp,
        ];

        if context.parameters.remote_control.enable {
            actions.insert(0, Action::RemoteControl);
        }

        match world_state.robot.role {
            Role::Defender => match world_state.filtered_game_controller_state {
                Some(FilteredGameControllerState {
                    sub_state: Some(SubState::CornerKick),
                    kicking_team: Some(Team::Opponent),
                    ..
                }) => {
                    let side = match world_state.rule_ball {
                        Some(ball) => ball.field_side,
                        None => Side::Left,
                    };
                    actions.push(Action::DefendOpponentCornerKick { side })
                }
                _ => actions.push(Action::DefendLeft),
            },
            Role::Keeper => actions.push(Action::DefendGoal),
            Role::Loser => actions.push(Action::SearchForLostBall),
            Role::Midfielder => {
                let side = match world_state.rule_ball {
                    Some(ball) => ball.field_side,
                    None => Side::Left,
                };
                match side {
                    Side::Left => actions.push(Action::SupportLeft),
                    Side::Right => actions.push(Action::SupportRight),
                }
            }
            Role::ReplacementKeeper => actions.push(Action::DefendGoal),

            Role::Searcher => actions.push(Action::Search),
            Role::Striker => match world_state.filtered_game_controller_state {
                None
                | Some(FilteredGameControllerState {
                    game_state:
                        FilteredGameState::Playing {
                            ball_is_free: true, ..
                        },
                    ..
                }) => {
                    actions.push(Action::Kicking);
                }
                Some(FilteredGameControllerState {
                    game_state: FilteredGameState::Ready,
                    kicking_team: Some(Team::Hulks),
                    sub_state,
                    ..
                }) => match sub_state {
                    Some(SubState::PenaltyKick) => actions.push(Action::WalkToPenaltyKick),
                    _ => actions.push(Action::WalkToKickOff),
                },
                Some(FilteredGameControllerState {
                    game_state: FilteredGameState::Ready | FilteredGameState::Playing { .. },
                    sub_state: Some(SubState::PenaltyKick),
                    kicking_team: Some(Team::Opponent),
                    ..
                }) => actions.push(Action::DefendPenaltyKick),
                _ => actions.push(Action::DefendKickOff),
            },
            Role::StrikerSupporter => actions.push(Action::SupportStriker),
        };

        // if world_state.robot.primary_state == PrimaryState::Playing {
        //     actions.push(Action::WalkToBall)
        //};
        let walk_path_planner = WalkPathPlanner::new(
            context.field_dimensions,
            &world_state.obstacles,
            &context.parameters.path_planning,
            context.last_motion_command,
        );
        let walk_and_stand = WalkAndStand::new(
            world_state,
            &context.parameters.walk_and_stand,
            &walk_path_planner,
            context.last_motion_command,
        );
        let look_action = LookAction::new(world_state);
        let mut defend = Defend::new(
            world_state,
            context.field_dimensions,
            &context.parameters.role_positions,
            &walk_and_stand,
            &look_action,
            &mut self.last_defender_mode,
        );
        let (action, motion_command) = actions
            .iter()
            .find_map(|action| {
                let motion_command = match action {
                    Action::Safe => safe::execute(world_state),
                    Action::Stop => stop::execute(world_state),
                    Action::Penalize => penalize::execute(world_state),
                    Action::Initial => initial::execute(world_state),
                    Action::Finish => finish::execute(world_state),
                    Action::StandUp => stand_up::execute(world_state),
                    Action::LookAround => look_around::execute(world_state),
                    Action::RemoteControl => {
                        remote_control::execute(&context.parameters.remote_control)
                    }
                    Action::DefendGoal => defend.goal(
                        &mut context.path_obstacles_output,
                        context.walk_speed.defend,
                        context
                            .parameters
                            .walk_and_stand
                            .defender_distance_to_be_aligned,
                    ),
                    Action::DefendKickOff => defend.kick_off(
                        &mut context.path_obstacles_output,
                        context.walk_speed.defend,
                        context
                            .parameters
                            .walk_and_stand
                            .defender_distance_to_be_aligned,
                    ),
                    Action::DefendLeft => defend.left(
                        &mut context.path_obstacles_output,
                        context.walk_speed.defend,
                        context
                            .parameters
                            .walk_and_stand
                            .defender_distance_to_be_aligned,
                    ),
                    Action::DefendPenaltyKick => defend.penalty_kick(
                        &mut context.path_obstacles_output,
                        context.walk_speed.defend,
                        context
                            .parameters
                            .walk_and_stand
                            .defender_distance_to_be_aligned,
                    ),
                    Action::DefendOpponentCornerKick { side: Side::Left } => defend
                        .opponent_corner_kick(
                            &mut context.path_obstacles_output,
                            context.walk_speed.defend,
                            Side::Left,
                            context
                                .parameters
                                .walk_and_stand
                                .defender_distance_to_be_aligned,
                        ),
                    Action::DefendOpponentCornerKick { side: Side::Right } => defend
                        .opponent_corner_kick(
                            &mut context.path_obstacles_output,
                            context.walk_speed.defend,
                            Side::Right,
                            context
                                .parameters
                                .walk_and_stand
                                .defender_distance_to_be_aligned,
                        ),
                    Action::StandDuringPenaltyKick => stand_during_penalty_kick::execute(
                        world_state,
                        context.field_dimensions,
                        &context.world_state.robot.role,
                    ),
                    Action::Kicking => kicking::execute(
                        world_state,
                        &walk_path_planner,
                        &context.parameters.kicking,
                        context.walk_speed.kicking,
                        context
                            .parameters
                            .walk_and_stand
                            .normal_distance_to_be_aligned,
                        *context.field_dimensions,
                        &mut context.path_obstacles_output,
                    ),
                    Action::Search => search::execute(
                        world_state,
                        &walk_path_planner,
                        &walk_and_stand,
                        context.field_dimensions,
                        &context.parameters.search,
                        &mut context.path_obstacles_output,
                        self.previous_role,
                        self.last_time_role_changed,
                        self.last_known_ball_position,
                        context.walk_speed.search,
                        context
                            .parameters
                            .walk_and_stand
                            .normal_distance_to_be_aligned,
                        context.cycle_time.start_time,
                    ),
                    Action::SearchForLostBall => lost_ball::execute(
                        world_state,
                        self.last_known_ball_position,
                        &walk_path_planner,
                        &context.parameters.lost_ball,
                        &mut context.path_obstacles_output,
                        context.walk_speed.lost_ball,
                        context
                            .parameters
                            .walk_and_stand
                            .normal_distance_to_be_aligned,
                    ),
                    Action::SupportLeft => support::execute(
                        world_state,
                        context.field_dimensions,
                        Some(Side::Left),
                        context
                            .parameters
                            .role_positions
                            .left_midfielder_distance_to_ball,
                        context
                            .parameters
                            .role_positions
                            .left_midfielder_maximum_x_in_ready_and_when_ball_is_not_free,
                        context.parameters.role_positions.left_midfielder_minimum_x,
                        &walk_and_stand,
                        &look_action,
                        &mut context.path_obstacles_output,
                        context.walk_speed.support,
                        context
                            .parameters
                            .walk_and_stand
                            .normal_distance_to_be_aligned,
                    ),
                    Action::SupportRight => support::execute(
                        world_state,
                        context.field_dimensions,
                        Some(Side::Right),
                        context
                            .parameters
                            .role_positions
                            .right_midfielder_distance_to_ball,
                        context
                            .parameters
                            .role_positions
                            .right_midfielder_maximum_x_in_ready_and_when_ball_is_not_free,
                        context.parameters.role_positions.right_midfielder_minimum_x,
                        &walk_and_stand,
                        &look_action,
                        &mut context.path_obstacles_output,
                        context.walk_speed.support,
                        context
                            .parameters
                            .walk_and_stand
                            .normal_distance_to_be_aligned,
                    ),
                    Action::SupportStriker => support::execute(
                        world_state,
                        context.field_dimensions,
                        None,
                        context
                            .parameters
                            .role_positions
                            .striker_supporter_distance_to_ball,
                        context
                            .parameters
                            .role_positions
                            .striker_supporter_maximum_x_in_ready_and_when_ball_is_not_free,
                        context
                            .parameters
                            .role_positions
                            .striker_supporter_minimum_x,
                        &walk_and_stand,
                        &look_action,
                        &mut context.path_obstacles_output,
                        context.walk_speed.support,
                        context
                            .parameters
                            .walk_and_stand
                            .normal_distance_to_be_aligned,
                    ),
                    Action::WalkToKickOff => walk_to_kick_off::execute(
                        world_state,
                        &walk_and_stand,
                        &look_action,
                        &mut context.path_obstacles_output,
                        context.parameters.role_positions.striker_kickoff_position,
                        context.kick_decision_parameters.kick_off_angle,
                        context.walk_speed.walk_to_kickoff,
                        context
                            .parameters
                            .walk_and_stand
                            .normal_distance_to_be_aligned,
                    ),
                    Action::WalkToPenaltyKick => walk_to_penalty_kick::execute(
                        world_state,
                        &walk_and_stand,
                        &look_action,
                        &mut context.path_obstacles_output,
                        context.field_dimensions,
                        context.walk_speed.walk_to_penalty_kick,
                        context
                            .parameters
                            .walk_and_stand
                            .normal_distance_to_be_aligned,
                    ),

                    Action::WalkToBall => walk_to_ball::execute(
                        context.ball_position.copied(),
                        context.parameters.walk_with_velocity.clone(),
                    ),
                    Action::VisualKick => {
                        visual_kick::execute(world_state, context.last_motion_command)
                    }
                }?;
                Some((action, motion_command))
            })
            .unwrap_or_else(|| panic!("there has to be at least one action available",));

        context.active_action.fill_if_subscribed(|| *action);

        *context.last_motion_command = motion_command.clone();

        Ok(MainOutputs {
            motion_command: motion_command.into(),
        })
    }
}
