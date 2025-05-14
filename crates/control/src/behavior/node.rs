use std::time::SystemTime;

use color_eyre::Result;
use serde::{Deserialize, Serialize};

use context_attribute::context;
use coordinate_systems::Field;
use framework::{AdditionalOutput, MainOutput};
use linear_algebra::{point, Point2};
use spl_network_messages::{GamePhase, PlayerNumber, SubState, Team};
use types::{
    action::Action,
    cycle_time::CycleTime,
    field_dimensions::{FieldDimensions, GlobalFieldSide, Side},
    filtered_game_controller_state::FilteredGameControllerState,
    filtered_game_state::FilteredGameState,
    kick_decision::DecisionParameters,
    motion_command::{MotionCommand, OrientationMode, WalkSpeed},
    parameters::{
        BehaviorParameters, InWalkKicksParameters, InterceptBallParameters, KeeperMotionParameters,
        LostBallParameters,
    },
    path_obstacles::PathObstacle,
    planned_path::PathSegment,
    primary_state::PrimaryState,
    roles::Role,
    step::Step,
    world_state::WorldState,
};

use super::{
    animation, calibrate,
    defend::{Defend, DefendMode},
    dribble, fall_safely,
    head::LookAction,
    initial, intercept_ball, jump, look_around, look_at_referee, lost_ball, no_ground_contact,
    penalize, prepare_jump, search, sit_down, stand, stand_up, support, unstiff, walk_to_kick_off,
    walk_to_penalty_kick,
    walk_to_pose::{WalkAndStand, WalkPathPlanner},
};

#[derive(Deserialize, Serialize)]
pub struct Behavior {
    last_known_ball_position: Point2<Field>,
    active_since: Option<SystemTime>,
    previous_role: Role,
    last_defender_mode: DefendMode,
}

#[context]
pub struct CreationContext {}

#[context]
pub struct CycleContext {
    has_ground_contact: Input<bool, "has_ground_contact">,
    world_state: Input<WorldState, "world_state">,
    dribble_path_plan: Input<Option<(OrientationMode, Vec<PathSegment>)>, "dribble_path_plan?">,
    cycle_time: Input<CycleTime, "cycle_time">,
    is_localization_converged: Input<bool, "is_localization_converged">,
    expected_referee_position: Input<Option<Point2<Field>>, "expected_referee_position?">,

    parameters: Parameter<BehaviorParameters, "behavior">,
    kick_decision_parameters: Parameter<DecisionParameters, "kick_selector">,
    in_walk_kicks: Parameter<InWalkKicksParameters, "in_walk_kicks">,
    field_dimensions: Parameter<FieldDimensions, "field_dimensions">,
    lost_ball_parameters: Parameter<LostBallParameters, "behavior.lost_ball">,
    intercept_ball_parameters: Parameter<InterceptBallParameters, "behavior.intercept_ball">,
    maximum_step_size: Parameter<Step, "step_planner.max_step_size">,
    enable_pose_detection: Parameter<bool, "pose_detection.enable">,
    keeper_motion: Parameter<KeeperMotionParameters, "keeper_motion">,
    use_stand_head_unstiff_calibration:
        Parameter<bool, "calibration_controller.use_stand_head_unstiff_calibration">,

    defend_walk_speed: Parameter<WalkSpeed, "walk_speed.defend">,
    dribble_walk_speed: Parameter<WalkSpeed, "walk_speed.dribble">,
    intercept_ball_walk_speed: Parameter<WalkSpeed, "walk_speed.intercept_ball">,
    lost_ball_walk_speed: Parameter<WalkSpeed, "walk_speed.lost_ball">,
    search_walk_speed: Parameter<WalkSpeed, "walk_speed.search">,
    support_walk_speed: Parameter<WalkSpeed, "walk_speed.support">,
    walk_to_kickoff_walk_speed: Parameter<WalkSpeed, "walk_speed.walk_to_kickoff">,
    walk_to_penalty_kick_walk_speed: Parameter<WalkSpeed, "walk_speed.walk_to_penalty_kick">,

    path_obstacles_output: AdditionalOutput<Vec<PathObstacle>, "path_obstacles">,
    active_action_output: AdditionalOutput<Action, "active_action">,

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
            last_known_ball_position: point![0.0, 0.0],
            active_since: None,
            previous_role: Role::Searcher,
            last_defender_mode: DefendMode::Passive,
        })
    }

    pub fn cycle(&mut self, mut context: CycleContext) -> Result<MainOutputs> {
        let world_state = context.world_state;
        if let Some(command) = &context.parameters.injected_motion_command {
            return Ok(MainOutputs {
                motion_command: command.clone().into(),
            });
        }

        if let Some(ball_state) = &world_state.ball {
            self.last_known_ball_position = ball_state.ball_in_field;
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

        if self.previous_role != context.world_state.robot.role
            && context.world_state.robot.role != Role::Searcher
            && context.world_state.robot.role != Role::Loser
            && self.previous_role != Role::Keeper
        {
            self.previous_role = context.world_state.robot.role;
        }

        let mut actions = vec![
            Action::Unstiff,
            Action::Animation,
            Action::SitDown,
            Action::Penalize,
            Action::Initial,
            Action::FallSafely,
            Action::StandUp,
            Action::NoGroundContact,
            Action::Stand,
            Action::Calibrate,
        ];

        if let Some(active_since) = self.active_since {
            let duration_active = now.duration_since(active_since)?;
            if !context.is_localization_converged
                && (duration_active < context.parameters.maximum_lookaround_duration)
            {
                actions.push(Action::LookAround);
            }
        }

        if matches!(world_state.robot.player_number, PlayerNumber::One) {
            actions.push(Action::KeeperMotion);
        }
        actions.push(Action::InterceptBall);

        match world_state.robot.role {
            Role::DefenderLeft if should_do_kick_in_pose_detection(world_state) => {
                actions.push(Action::LookAtReferee);
                actions.push(Action::DefendLeft);
            }
            Role::DefenderLeft => match world_state.filtered_game_controller_state {
                Some(FilteredGameControllerState {
                    sub_state: Some(SubState::CornerKick),
                    kicking_team: Some(Team::Opponent),
                    ..
                }) => actions.push(Action::DefendOpponentCornerKick { side: Side::Left }),
                _ => actions.push(Action::DefendLeft),
            },
            Role::DefenderRight if should_do_kick_in_pose_detection(world_state) => {
                actions.push(Action::LookAtReferee);
                actions.push(Action::DefendRight);
            }
            Role::DefenderRight => match world_state.filtered_game_controller_state {
                Some(FilteredGameControllerState {
                    sub_state: Some(SubState::CornerKick),
                    kicking_team: Some(Team::Opponent),
                    ..
                }) => actions.push(Action::DefendOpponentCornerKick { side: Side::Right }),
                _ => actions.push(Action::DefendRight),
            },
            Role::Keeper => match world_state.filtered_game_controller_state {
                Some(FilteredGameControllerState {
                    game_phase: GamePhase::PenaltyShootout { .. },
                    ..
                })
                | Some(FilteredGameControllerState {
                    game_state: FilteredGameState::Playing { .. },
                    kicking_team: Some(Team::Opponent),
                    sub_state: Some(SubState::PenaltyKick),
                    ..
                }) => {
                    actions.push(Action::Jump);
                    actions.push(Action::PrepareJump);
                }
                _ => actions.push(Action::DefendGoal),
            },
            Role::Loser => actions.push(Action::SearchForLostBall),
            Role::MidfielderLeft if should_do_kick_in_pose_detection(world_state) => {
                actions.push(Action::LookAtReferee);
                actions.push(Action::SupportLeft);
            }
            Role::MidfielderLeft => actions.push(Action::SupportLeft),
            Role::MidfielderRight if should_do_kick_in_pose_detection(world_state) => {
                actions.push(Action::LookAtReferee);
                actions.push(Action::SupportRight);
            }
            Role::MidfielderRight => actions.push(Action::SupportRight),
            Role::ReplacementKeeper => actions.push(Action::DefendGoal),
            Role::Searcher if should_do_kick_in_pose_detection(world_state) => {
                actions.push(Action::LookAtReferee);
                actions.push(Action::Search);
            }
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
                    actions.push(Action::Dribble);
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
                    Action::Animation => animation::execute(world_state),
                    Action::Unstiff => unstiff::execute(world_state),
                    Action::SitDown => sit_down::execute(world_state),
                    Action::Penalize => penalize::execute(world_state),
                    Action::Initial => {
                        initial::execute(world_state, *context.enable_pose_detection)
                    }
                    Action::LookAtReferee => look_at_referee::execute(
                        *context.enable_pose_detection,
                        &walk_and_stand,
                        context.expected_referee_position,
                        context.world_state,
                        &mut context.path_obstacles_output,
                        *context.support_walk_speed,
                        context
                            .parameters
                            .walk_and_stand
                            .normal_distance_to_be_aligned,
                    ),
                    Action::FallSafely => {
                        fall_safely::execute(world_state, *context.has_ground_contact)
                    }
                    Action::StandUp => stand_up::execute(world_state),
                    Action::NoGroundContact => no_ground_contact::execute(world_state),
                    Action::LookAround => look_around::execute(world_state),
                    Action::KeeperMotion => defend.keeper_motion(context.keeper_motion.clone()),
                    Action::InterceptBall => intercept_ball::execute(
                        world_state,
                        *context.intercept_ball_parameters,
                        *context.maximum_step_size,
                        *context.intercept_ball_walk_speed,
                    ),
                    Action::Calibrate => {
                        calibrate::execute(world_state, *context.use_stand_head_unstiff_calibration)
                    }
                    Action::DefendGoal => defend.goal(
                        &mut context.path_obstacles_output,
                        *context.defend_walk_speed,
                        context
                            .parameters
                            .walk_and_stand
                            .defender_distance_to_be_aligned,
                    ),
                    Action::DefendKickOff => defend.kick_off(
                        &mut context.path_obstacles_output,
                        *context.defend_walk_speed,
                        context
                            .parameters
                            .walk_and_stand
                            .defender_distance_to_be_aligned,
                    ),
                    Action::DefendLeft => defend.left(
                        &mut context.path_obstacles_output,
                        *context.defend_walk_speed,
                        context
                            .parameters
                            .walk_and_stand
                            .defender_distance_to_be_aligned,
                    ),
                    Action::DefendRight => defend.right(
                        &mut context.path_obstacles_output,
                        *context.defend_walk_speed,
                        context
                            .parameters
                            .walk_and_stand
                            .defender_distance_to_be_aligned,
                    ),
                    Action::DefendPenaltyKick => defend.penalty_kick(
                        &mut context.path_obstacles_output,
                        *context.defend_walk_speed,
                        context
                            .parameters
                            .walk_and_stand
                            .defender_distance_to_be_aligned,
                    ),
                    Action::DefendOpponentCornerKick { side: Side::Left } => defend
                        .opponent_corner_kick(
                            &mut context.path_obstacles_output,
                            *context.defend_walk_speed,
                            Side::Left,
                            context
                                .parameters
                                .walk_and_stand
                                .defender_distance_to_be_aligned,
                        ),
                    Action::DefendOpponentCornerKick { side: Side::Right } => defend
                        .opponent_corner_kick(
                            &mut context.path_obstacles_output,
                            *context.defend_walk_speed,
                            Side::Right,
                            context
                                .parameters
                                .walk_and_stand
                                .defender_distance_to_be_aligned,
                        ),
                    Action::Stand => stand::execute(
                        world_state,
                        context.field_dimensions,
                        &context.world_state.robot.role,
                    ),
                    Action::Dribble => dribble::execute(
                        world_state,
                        &walk_path_planner,
                        context.in_walk_kicks,
                        &context.parameters.dribbling,
                        context.dribble_path_plan.cloned(),
                        *context.dribble_walk_speed,
                    ),
                    Action::Jump => jump::execute(world_state),
                    Action::PrepareJump => prepare_jump::execute(world_state),
                    Action::Search => search::execute(
                        world_state,
                        &walk_path_planner,
                        &walk_and_stand,
                        context.field_dimensions,
                        &context.parameters.search,
                        &mut context.path_obstacles_output,
                        self.previous_role,
                        *context.search_walk_speed,
                        context
                            .parameters
                            .walk_and_stand
                            .normal_distance_to_be_aligned,
                    ),
                    Action::SearchForLostBall => lost_ball::execute(
                        world_state,
                        self.last_known_ball_position,
                        &walk_path_planner,
                        context.lost_ball_parameters,
                        &mut context.path_obstacles_output,
                        *context.lost_ball_walk_speed,
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
                        *context.support_walk_speed,
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
                        *context.support_walk_speed,
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
                        *context.support_walk_speed,
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
                        *context.walk_to_kickoff_walk_speed,
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
                        *context.walk_to_penalty_kick_walk_speed,
                        context
                            .parameters
                            .walk_and_stand
                            .normal_distance_to_be_aligned,
                    ),
                }?;
                Some((action, motion_command))
            })
            .unwrap_or_else(|| {
                panic!(
                    "there has to be at least one action available, world_state: {world_state:#?}",
                )
            });
        context.active_action_output.fill_if_subscribed(|| *action);

        *context.last_motion_command = motion_command.clone();

        Ok(MainOutputs {
            motion_command: motion_command.into(),
        })
    }
}

pub fn should_do_kick_in_pose_detection(world_state: &WorldState) -> bool {
    if let Some(filtered_game_controller_state) = &world_state.filtered_game_controller_state {
        let is_kick_in_filtered_game_controller_state = matches!(
            filtered_game_controller_state,
            FilteredGameControllerState {
                sub_state: Some(SubState::KickIn),
                game_state: FilteredGameState::Playing {
                    ball_is_free: false,
                    ..
                },
                kicking_team: Some(..),
                ..
            }
        );

        let first_two_nonpenalized_nonkeeper_player_numbers: Vec<PlayerNumber> =
            filtered_game_controller_state
                .penalties
                .iter()
                .filter_map(|(player_number, penalty)| penalty.is_none().then_some(player_number))
                // Skip the lowest non-penalized player number since this is always the Keeper or ReplacementKeeper
                .skip(1)
                .take(2)
                .collect();

        let is_correct_kick_in_detection_role = match (
            world_state.robot.role,
            filtered_game_controller_state.global_field_side,
        ) {
            (Role::DefenderRight | Role::MidfielderRight, GlobalFieldSide::Home) => true,
            (Role::DefenderLeft | Role::MidfielderLeft, GlobalFieldSide::Away) => true,
            (Role::Searcher, _)
                if first_two_nonpenalized_nonkeeper_player_numbers
                    .contains(&world_state.robot.player_number) =>
            {
                true
            }
            _ => false,
        };

        is_kick_in_filtered_game_controller_state && is_correct_kick_in_detection_role
    } else {
        false
    }
}
