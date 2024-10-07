use std::{
    cmp::Ordering,
    net::SocketAddr,
    time::{Duration, SystemTime},
};

use color_eyre::{eyre::WrapErr, Result};
use serde::{Deserialize, Serialize};

use context_attribute::context;
use coordinate_systems::{Field, Ground};
use framework::{MainOutput, PerceptionInput};
use hardware::NetworkInterface;
use linear_algebra::{Isometry2, Point2, Vector};
use spl_network_messages::{
    GameControllerReturnMessage, GamePhase, HulkMessage, StrikerMessage, SubState, Team,
};
use types::{
    ball_position::BallPosition,
    cycle_time::CycleTime,
    fall_state::FallState,
    field_dimensions::FieldDimensions,
    filtered_game_controller_state::FilteredGameControllerState,
    initial_pose::InitialPose,
    messages::{IncomingMessage, OutgoingMessage},
    parameters::SplNetworkParameters,
    primary_state::PrimaryState,
    roles::Role,
};

use crate::localization::generate_initial_pose;

#[derive(Deserialize, Serialize)]
pub struct RoleAssignment {
    last_received_spl_striker_message: Option<SystemTime>,
    last_system_time_transmitted_game_controller_return_message: Option<SystemTime>,
    last_transmitted_spl_striker_message: Option<SystemTime>,
    role: Role,
    role_initialized: bool,
    team_ball: Option<BallPosition<Field>>,
    // last_time_player_was_penalized: Players<Option<SystemTime>>,
}

#[context]
pub struct CreationContext {}

#[context]
pub struct CycleContext {
    ball_position: Input<Option<BallPosition<Ground>>, "ball_position?">,
    cycle_time: Input<CycleTime, "cycle_time">,
    fall_state: Input<FallState, "fall_state">,
    filtered_game_controller_state:
        Input<Option<FilteredGameControllerState>, "filtered_game_controller_state?">,
    game_controller_address: Input<Option<SocketAddr>, "game_controller_address?">,
    ground_to_field: Input<Option<Isometry2<Ground, Field>>, "ground_to_field?">,
    network_message: PerceptionInput<Option<IncomingMessage>, "SplNetwork", "filtered_message?">,
    primary_state: Input<PrimaryState, "primary_state">,
    replacement_keeper_priority: Input<Option<usize>, "replacement_keeper_priority?">,
    striker_priority: Input<Option<usize>, "striker_priority?">,
    time_to_reach_kick_position: CyclerState<Duration, "time_to_reach_kick_position">,
    walk_in_position_index: Input<usize, "walk_in_position_index">,

    field_dimensions: Parameter<FieldDimensions, "field_dimensions">,
    forced_role: Parameter<Option<Role>, "role_assignment.forced_role?">,
    _keeper_replacementkeeper_switch_time:
        Parameter<Duration, "role_assignment.keeper_replacementkeeper_switch_time">,
    initial_poses: Parameter<Vec<InitialPose>, "localization.initial_poses">,
    offense_optional_roles: Parameter<Vec<Role>, "behavior.offense_optional_roles">,
    number_of_defensive_players: Parameter<usize, "behavior.number_of_defensive_players">,
    jersey_number: Parameter<usize, "jersey_number">,
    spl_network: Parameter<SplNetworkParameters, "spl_network">,

    hardware: HardwareInterface,
    // last_time_player_was_penalized:
    //     AdditionalOutput<Players<Option<SystemTime>>, "last_time_player_penalized">,
}

#[context]
#[derive(Default)]
pub struct MainOutputs {
    pub team_ball: MainOutput<Option<BallPosition<Field>>>,
    pub network_robot_obstacles: MainOutput<Vec<Point2<Ground>>>,
    pub role: MainOutput<Role>,
}

impl RoleAssignment {
    pub fn new(_context: CreationContext) -> Result<Self> {
        Ok(Self {
            last_received_spl_striker_message: None,
            last_system_time_transmitted_game_controller_return_message: None,
            last_transmitted_spl_striker_message: None,
            role: Role::Striker,
            role_initialized: false,
            team_ball: None,
            // last_time_player_was_penalized: Players {
            //     one: None,
            //     two: None,
            //     three: None,
            //     four: None,
            //     five: None,
            //     six: None,
            //     seven: None,
            // },
        })
    }

    pub fn cycle(&mut self, context: CycleContext<impl NetworkInterface>) -> Result<MainOutputs> {
        let cycle_start_time = context.cycle_time.start_time;
        let primary_state = *context.primary_state;
        let mut new_role = self.role;

        let ground_to_field =
            context
                .ground_to_field
                .copied()
                .unwrap_or_else(|| match context.primary_state {
                    PrimaryState::Initial => generate_initial_pose(
                        &context.initial_poses[*context.walk_in_position_index],
                        context.field_dimensions,
                    )
                    .as_transform(),
                    _ => Default::default(),
                });

        let mut defense_optional_roles = context.offense_optional_roles.clone();
        defense_optional_roles.reverse();
        // let available_field_players = if let Some(game_controller_state) =
        //     context.filtered_game_controller_state
        // {
        //     game_controller_state
        //         .penalties
        //         .iter()
        //         .filter(|(&jersey_number, penalty)| {
        //             penalty.is_none() && jersey_number != game_controller_state.goal_keeper_number
        //         })
        //         .map(|(&jersey_number, _)| jersey_number)
        //         .sorted()
        //         .collect::<Vec<_>>()
        // } else {
        //     Vec::new()
        // };
        // let replacement_keeper_order_position = available_field_players
        //     .iter()
        //     .position(|&jersey_number| jersey_number == *context.jersey_number);
        // let striker_assignment_order_position = available_field_players
        //     .iter()
        //     .rev()
        //     .position(|&jersey_number| jersey_number == *context.jersey_number);

        if !self.role_initialized
            || primary_state == PrimaryState::Ready
            || primary_state == PrimaryState::Set
        {
            if let Some(game_controller_state) = context.filtered_game_controller_state {
                if let Some(0) = context.replacement_keeper_priority {
                    new_role = Role::ReplacementKeeper;
                } else if let Some(0) = context.striker_priority {
                    new_role = Role::Striker;
                } else if game_controller_state.goal_keeper_number == *context.jersey_number {
                    new_role = Role::Keeper;
                } else {
                    new_role = match (
                        context.striker_priority,
                        context.replacement_keeper_priority,
                    ) {
                        (None, None) => Default::default(),
                        (None, Some(replacement_keeper_index)) => defense_optional_roles
                            .get(replacement_keeper_index - 1)
                            .copied()
                            .unwrap_or_default(),
                        (Some(striker_index), None) => context
                            .offense_optional_roles
                            .get(striker_index - 1)
                            .copied()
                            .unwrap_or_default(),
                        (Some(striker_index), Some(replacement_keeper_index)) => {
                            if replacement_keeper_index <= context.number_of_defensive_players {
                                defense_optional_roles
                                    .get(replacement_keeper_index - 1)
                                    .copied()
                                    .unwrap_or_default()
                            } else {
                                context
                                    .offense_optional_roles
                                    .get(striker_index - 1)
                                    .copied()
                                    .unwrap_or_default()
                            }
                        }
                    };
                }
            }

            // if let Some(game_controller_state) = context.filtered_game_controller_state {
            //     if let Some(striker) = [
            //         PlayerNumber::Seven,
            //         PlayerNumber::Six,
            //         PlayerNumber::Five,
            //         PlayerNumber::Four,
            //     ]
            //     .into_iter()
            //     .find(|player| game_controller_state.penalties[*player].is_none())
            //     {
            //         player_roles[striker] = Role::Striker;
            //     }
            // }
            // new_role = player_roles[*context.player_number];

            self.role_initialized = true;
            self.last_received_spl_striker_message = Some(cycle_start_time);
            self.team_ball = None;
        }

        let send_game_controller_return_message = self
            .last_system_time_transmitted_game_controller_return_message
            .is_none()
            || cycle_start_time.duration_since(
                self.last_system_time_transmitted_game_controller_return_message
                    .unwrap(),
            )? > context.spl_network.game_controller_return_message_interval;

        let mut send_spl_striker_message = self.last_transmitted_spl_striker_message.is_none()
            || cycle_start_time
                .duration_since(self.last_transmitted_spl_striker_message.unwrap())?
                > context.spl_network.spl_striker_message_send_interval;

        let spl_striker_message_timeout = match self.last_received_spl_striker_message {
            None => false,
            Some(last_received_spl_striker_message) => {
                cycle_start_time.duration_since(last_received_spl_striker_message)?
                    > context.spl_network.spl_striker_message_receive_timeout
            }
        };

        let silence_interval_has_passed = match self.last_transmitted_spl_striker_message {
            Some(last_transmitted_spl_striker_message) => {
                cycle_start_time.duration_since(last_transmitted_spl_striker_message)?
                    > context.spl_network.silence_interval_between_messages
            }
            None => true,
        };

        if send_game_controller_return_message {
            self.last_system_time_transmitted_game_controller_return_message =
                Some(cycle_start_time);
            if let Some(address) = context.game_controller_address {
                context
                    .hardware
                    .write_to_network(OutgoingMessage::GameController(
                        *address,
                        GameControllerReturnMessage {
                            jersey_number: *context.jersey_number,
                            fallen: matches!(context.fall_state, FallState::Fallen { .. }),
                            pose: ground_to_field.as_pose(),
                            ball: seen_ball_to_game_controller_ball_position(
                                context.ball_position,
                                cycle_start_time,
                            ),
                        },
                    ))
                    .wrap_err("failed to write GameControllerReturnMessage to hardware")?;
            }
        }

        let mut team_ball = self.team_ball;

        let is_in_penalty_kick = matches!(
            context.filtered_game_controller_state,
            Some(FilteredGameControllerState {
                sub_state: Some(SubState::PenaltyKick),
                ..
            })
        );
        if spl_striker_message_timeout && !is_in_penalty_kick {
            match new_role {
                Role::Keeper => {
                    team_ball = None;
                }
                Role::ReplacementKeeper => {
                    team_ball = None;
                }
                Role::Striker => {
                    send_spl_striker_message = true;
                    team_ball = None;
                    new_role = Role::Loser;
                }
                Role::Loser => {
                    if let Some(game_controller_state) = context.filtered_game_controller_state {
                        if game_controller_state.goal_keeper_number == *context.jersey_number {
                            new_role = Role::Keeper
                        }
                    }
                }
                _ => {
                    send_spl_striker_message = false;
                    team_ball = None;
                    new_role = Role::Searcher
                }
            }
        }

        let mut network_robot_obstacles = vec![];
        let mut spl_messages = context
            .network_message
            .persistent
            .into_values()
            .flatten()
            .filter_map(|message| match message {
                Some(IncomingMessage::Spl(HulkMessage::Striker(message))) => Some(message),
                _ => None,
            })
            .peekable();
        if spl_messages.peek().is_none() {
            (new_role, send_spl_striker_message, team_ball) = process_role_state_machine(
                new_role,
                ground_to_field,
                context.ball_position,
                primary_state,
                None,
                Some(*context.time_to_reach_kick_position),
                send_spl_striker_message,
                team_ball,
                cycle_start_time,
                context.filtered_game_controller_state,
                context.spl_network.striker_trusts_team_ball,
                context.offense_optional_roles,
                &defense_optional_roles,
                *context.number_of_defensive_players,
                context.replacement_keeper_priority.copied(),
                context.striker_priority.copied(),
                *context.jersey_number,
            );
        } else {
            for spl_message in spl_messages {
                self.last_received_spl_striker_message = Some(cycle_start_time);
                let sender_position = ground_to_field.inverse() * spl_message.pose.position();
                if spl_message.jersey_number != *context.jersey_number {
                    network_robot_obstacles.push(sender_position);
                }
                (new_role, send_spl_striker_message, team_ball) = process_role_state_machine(
                    new_role,
                    ground_to_field,
                    context.ball_position,
                    primary_state,
                    Some(spl_message),
                    Some(*context.time_to_reach_kick_position),
                    send_spl_striker_message,
                    team_ball,
                    cycle_start_time,
                    context.filtered_game_controller_state,
                    context.spl_network.striker_trusts_team_ball,
                    context.offense_optional_roles,
                    &defense_optional_roles,
                    *context.number_of_defensive_players,
                    context.replacement_keeper_priority.copied(),
                    context.striker_priority.copied(),
                    *context.jersey_number,
                );
            }
        }
        // todo! replace this with new indexing and figure out how to do since_last_penalized
        // if self.role == Role::ReplacementKeeper {
        //     let mut other_players_with_lower_number = self
        //         .last_time_player_was_penalized
        //         .iter()
        //         .filter(|(player_number, _)| player_number < context.player_number);
        //     let is_lowest_number_without =
        //         other_players_with_lower_number.all(|(_, penalized_time)| {
        //             penalized_time
        //                 .map(|system_time| {
        //                     let since_last_penalized = cycle_start_time
        //                         .duration_since(system_time)
        //                         .expect("penalty time to be in the past");
        //                     since_last_penalized < *context.keeper_replacementkeeper_switch_time
        //                 })
        //                 .unwrap_or(false)
        //         });
        //     if !send_spl_striker_message && is_lowest_number_without {
        //         new_role = Role::ReplacementKeeper;
        //     }
        // }
        // context
        //     .last_time_player_was_penalized
        //     .fill_if_subscribed(|| self.last_time_player_was_penalized);

        if send_spl_striker_message
            && primary_state == PrimaryState::Playing
            && silence_interval_has_passed
        {
            self.last_transmitted_spl_striker_message = Some(cycle_start_time);
            self.last_received_spl_striker_message = Some(cycle_start_time);
            if let Some(game_controller_state) = context.filtered_game_controller_state {
                if game_controller_state.remaining_number_of_messages
                    > context
                        .spl_network
                        .remaining_amount_of_messages_to_stop_sending
                {
                    let ball_position = if context.ball_position.is_none() && team_ball.is_some() {
                        team_ball_to_network_ball_position(team_ball, cycle_start_time)
                    } else {
                        seen_ball_to_hulks_network_ball_position(
                            context.ball_position,
                            ground_to_field,
                            cycle_start_time,
                        )
                    };
                    context.hardware.write_to_network(OutgoingMessage::Spl(
                        HulkMessage::Striker(StrikerMessage {
                            jersey_number: *context.jersey_number,
                            pose: ground_to_field.as_pose(),
                            ball_position,
                            time_to_reach_kick_position: Some(*context.time_to_reach_kick_position),
                        }),
                    ))?;
                }
            }
        }

        if let Some(forced_role) = context.forced_role {
            self.role = *forced_role;
        } else {
            self.role = new_role;
        }
        self.team_ball = team_ball;

        // if let Some(game_controller_state) = context.filtered_game_controller_state {
        //     for player in self
        //         .last_time_player_was_penalized
        //         .clone()
        //         .iter()
        //         .map(|(playernumber, ..)| playernumber)
        //     {
        //         if game_controller_state.penalties[player].is_some() {
        //             self.last_time_player_was_penalized[player] = Some(cycle_start_time);
        //         }
        //     }
        // }

        Ok(MainOutputs {
            role: self.role.into(),
            team_ball: self.team_ball.into(),
            network_robot_obstacles: network_robot_obstacles.into(),
        })
    }
}

#[allow(clippy::too_many_arguments)]
fn process_role_state_machine(
    current_role: Role,
    current_pose: Isometry2<Ground, Field>,
    detected_own_ball: Option<&BallPosition<Ground>>,
    primary_state: PrimaryState,
    incoming_message: Option<&StrikerMessage>,
    time_to_reach_kick_position: Option<Duration>,
    send_spl_striker_message: bool,
    team_ball: Option<BallPosition<Field>>,
    cycle_start_time: SystemTime,
    filtered_game_controller_state: Option<&FilteredGameControllerState>,
    striker_trusts_team_ball: Duration,
    offense_optional_roles: &[Role],
    defense_optional_roles: &[Role],
    number_of_defensive_players: usize,
    replacement_keeper_priority: Option<usize>,
    striker_priority: Option<usize>,
    own_jersey_number: usize,
) -> (Role, bool, Option<BallPosition<Field>>) {
    if let Some(game_controller_state) = filtered_game_controller_state {
        match game_controller_state.game_phase {
            GamePhase::PenaltyShootout {
                kicking_team: Team::Hulks,
            } => return (Role::Striker, false, None),
            GamePhase::PenaltyShootout {
                kicking_team: Team::Opponent,
            } => return (Role::Keeper, false, None),
            _ => {}
        };
        if let Some(SubState::PenaltyKick) = game_controller_state.sub_state {
            return (current_role, false, None);
        }
    }
    let goal_keeper_number = filtered_game_controller_state
        .map(|game_controller_state| game_controller_state.goal_keeper_number);

    if primary_state != PrimaryState::Playing {
        match detected_own_ball {
            None => return (current_role, false, team_ball),
            Some(..) => {
                return (
                    current_role,
                    false,
                    team_ball_from_seen_ball(detected_own_ball, current_pose, cycle_start_time),
                )
            }
        }
    }

    match (current_role, detected_own_ball, incoming_message) {
        //Striker maybe lost Ball
        (Role::Striker, None, None) => match team_ball {
            None => (Role::Loser, true, None),
            Some(team_ball) => {
                if cycle_start_time
                    .duration_since(team_ball.last_seen)
                    .unwrap()
                    > striker_trusts_team_ball
                {
                    (Role::Loser, true, None)
                } else {
                    (Role::Striker, send_spl_striker_message, Some(team_ball))
                }
            }
        },

        // Striker maybe lost Ball but got a message (edge-case)
        (Role::Striker, None, Some(spl_message)) => match &spl_message.ball_position {
            None => {
                // another Striker became Loser
                match team_ball {
                    None => (Role::Loser, true, None),
                    Some(team_ball) => {
                        if cycle_start_time
                            .duration_since(team_ball.last_seen)
                            .unwrap()
                            > striker_trusts_team_ball
                        {
                            (Role::Loser, true, None)
                        } else {
                            (Role::Striker, send_spl_striker_message, Some(team_ball))
                        }
                    }
                }
            }
            _ => decide_if_claiming_striker_or_other_role(
                spl_message,
                time_to_reach_kick_position,
                cycle_start_time,
                offense_optional_roles,
                defense_optional_roles,
                number_of_defensive_players,
                replacement_keeper_priority,
                striker_priority,
                goal_keeper_number,
                own_jersey_number,
            ),
        },

        //Striker remains Striker, sends message after timeout
        (Role::Striker, Some(..), None) => (Role::Striker, send_spl_striker_message, team_ball),

        // Striker got a message (either another Player claims Stiker role or Edge-case of a second Striker)
        (Role::Striker, Some(..), Some(spl_message)) => match &spl_message.ball_position {
            None => {
                // another Striker became Loser, so we claim striker since we see a ball
                (
                    Role::Striker,
                    true,
                    team_ball_from_seen_ball(detected_own_ball, current_pose, cycle_start_time),
                )
            }
            _ => decide_if_claiming_striker_or_other_role(
                spl_message,
                time_to_reach_kick_position,
                cycle_start_time,
                offense_optional_roles,
                defense_optional_roles,
                number_of_defensive_players,
                replacement_keeper_priority,
                striker_priority,
                goal_keeper_number,
                own_jersey_number,
            ),
        },

        //Loser remains Loser
        (Role::Loser, None, None) => (Role::Loser, false, team_ball),

        (Role::Loser, None, Some(spl_message)) => match &spl_message.ball_position {
            None => (Role::Loser, false, None), //edge-case, a striker (which should not exist) lost the ball
            _ => decide_if_claiming_striker_or_other_role(
                spl_message,
                time_to_reach_kick_position,
                cycle_start_time,
                offense_optional_roles,
                defense_optional_roles,
                number_of_defensive_players,
                replacement_keeper_priority,
                striker_priority,
                goal_keeper_number,
                own_jersey_number,
            ),
        },

        //Loser found ball and becomes Striker
        (Role::Loser, Some(..), None) => (
            Role::Striker,
            true,
            team_ball_from_seen_ball(detected_own_ball, current_pose, cycle_start_time),
        ),

        // Edge-case, Loser found Ball at the same time as receiving a message
        (Role::Loser, Some(..), Some(spl_message)) => match &spl_message.ball_position {
            None => {
                // another Striker became Loser, so we claim striker since we see a ball
                (
                    Role::Striker,
                    true,
                    team_ball_from_seen_ball(detected_own_ball, current_pose, cycle_start_time),
                )
            }
            _ => decide_if_claiming_striker_or_other_role(
                spl_message,
                time_to_reach_kick_position,
                cycle_start_time,
                offense_optional_roles,
                defense_optional_roles,
                number_of_defensive_players,
                replacement_keeper_priority,
                striker_priority,
                goal_keeper_number,
                own_jersey_number,
            ),
        },

        //Searcher remains Searcher
        (Role::Searcher, None, None) => (Role::Searcher, false, team_ball),

        (Role::Searcher, None, Some(spl_message)) => match &spl_message.ball_position {
            None => (Role::Searcher, false, team_ball), //edge-case, a striker (which should not exist) lost the ball
            _ => decide_if_claiming_striker_or_other_role(
                spl_message,
                time_to_reach_kick_position,
                cycle_start_time,
                offense_optional_roles,
                defense_optional_roles,
                number_of_defensive_players,
                replacement_keeper_priority,
                striker_priority,
                goal_keeper_number,
                own_jersey_number,
            ),
        },

        //Searcher found ball and becomes Striker
        (Role::Searcher, Some(..), None) => (
            Role::Striker,
            true,
            team_ball_from_seen_ball(detected_own_ball, current_pose, cycle_start_time),
        ),

        // TODO: Searcher found Ball at the same time as receiving a message
        (Role::Searcher, Some(..), Some(spl_message)) => match &spl_message.ball_position {
            None => (
                Role::Striker,
                true,
                team_ball_from_seen_ball(detected_own_ball, current_pose, cycle_start_time),
            ),
            _ => decide_if_claiming_striker_or_other_role(
                spl_message,
                time_to_reach_kick_position,
                cycle_start_time,
                offense_optional_roles,
                defense_optional_roles,
                number_of_defensive_players,
                replacement_keeper_priority,
                striker_priority,
                goal_keeper_number,
                own_jersey_number,
            ),
        },

        // remain in other_role
        (other_role, None, None) => (other_role, false, team_ball),

        // Either someone found or lost a ball. if found: do I want to claim striker ?
        (other_role, None, Some(spl_message)) => match &spl_message.ball_position {
            None => {
                if other_role != Role::Keeper && other_role != Role::ReplacementKeeper {
                    (Role::Searcher, false, None)
                } else {
                    (other_role, false, None)
                }
            }
            _ => decide_if_claiming_striker_or_other_role(
                spl_message,
                time_to_reach_kick_position,
                cycle_start_time,
                offense_optional_roles,
                defense_optional_roles,
                number_of_defensive_players,
                replacement_keeper_priority,
                striker_priority,
                goal_keeper_number,
                own_jersey_number,
            ),
        },

        // Claim Striker if team-ball position is None
        (other_role, Some(..), None) => match team_ball {
            None => (
                Role::Striker,
                true,
                team_ball_from_seen_ball(detected_own_ball, current_pose, cycle_start_time),
            ),
            Some(..) => (
                other_role,
                false,
                team_ball_from_seen_ball(detected_own_ball, current_pose, cycle_start_time),
            ),
        },

        // if message is Ball-Lost => Striker, claim Striker ? design-decision: which ball to trust ?
        (_other_role, Some(..), Some(spl_message)) => match &spl_message.ball_position {
            None => (
                Role::Striker,
                true,
                team_ball_from_seen_ball(detected_own_ball, current_pose, cycle_start_time),
            ),
            _ => decide_if_claiming_striker_or_other_role(
                spl_message,
                time_to_reach_kick_position,
                cycle_start_time,
                offense_optional_roles,
                defense_optional_roles,
                number_of_defensive_players,
                replacement_keeper_priority,
                striker_priority,
                goal_keeper_number,
                own_jersey_number,
            ),
        },
    }
}
#[allow(clippy::too_many_arguments)]
fn decide_if_claiming_striker_or_other_role(
    spl_message: &StrikerMessage,
    time_to_reach_kick_position: Option<Duration>,
    cycle_start_time: SystemTime,
    offense_optional_roles: &[Role],
    defense_optional_roles: &[Role],
    number_of_defensive_players: usize,
    replacement_keeper_priority: Option<usize>,
    striker_priority: Option<usize>,
    goal_keeper_number: Option<usize>,
    own_jersey_number: usize,
) -> (Role, bool, Option<BallPosition<Field>>) {
    if time_to_reach_kick_position < spl_message.time_to_reach_kick_position
        && time_to_reach_kick_position.is_some_and(|duration| duration < Duration::from_secs(1200))
    {
        (
            Role::Striker,
            true,
            team_ball_from_spl_message(cycle_start_time, spl_message),
        )
    } else {
        (
            generate_role(
                replacement_keeper_priority,
                striker_priority,
                goal_keeper_number,
                spl_message.jersey_number,
                own_jersey_number,
                offense_optional_roles,
                defense_optional_roles,
                number_of_defensive_players,
            ),
            false,
            team_ball_from_spl_message(cycle_start_time, spl_message),
        )
    }
}

fn seen_ball_to_game_controller_ball_position(
    ball: Option<&BallPosition<Ground>>,
    cycle_start_time: SystemTime,
) -> Option<spl_network_messages::BallPosition<Ground>> {
    ball.map(|ball| spl_network_messages::BallPosition {
        age: cycle_start_time.duration_since(ball.last_seen).unwrap(),
        position: ball.position,
    })
}

fn seen_ball_to_hulks_network_ball_position(
    ball: Option<&BallPosition<Ground>>,
    ground_to_field: Isometry2<Ground, Field>,
    cycle_start_time: SystemTime,
) -> Option<spl_network_messages::BallPosition<Field>> {
    ball.map(|ball| spl_network_messages::BallPosition {
        age: cycle_start_time.duration_since(ball.last_seen).unwrap(),
        position: ground_to_field * ball.position,
    })
}

fn team_ball_to_network_ball_position(
    team_ball: Option<BallPosition<Field>>,
    cycle_start_time: SystemTime,
) -> Option<spl_network_messages::BallPosition<Field>> {
    team_ball.map(|team_ball| spl_network_messages::BallPosition {
        age: cycle_start_time
            .duration_since(team_ball.last_seen)
            .unwrap(),
        position: team_ball.position,
    })
}

fn team_ball_from_spl_message(
    cycle_start_time: SystemTime,
    spl_message: &StrikerMessage,
) -> Option<BallPosition<Field>> {
    spl_message
        .ball_position
        .as_ref()
        .map(|ball_position| BallPosition {
            position: ball_position.position,
            velocity: Vector::zeros(),
            last_seen: cycle_start_time - ball_position.age,
        })
}

fn team_ball_from_seen_ball(
    ball: Option<&BallPosition<Ground>>,
    ground_to_field: Isometry2<Ground, Field>,
    cycle_start_time: SystemTime,
) -> Option<BallPosition<Field>> {
    ball.as_ref().map(|ball| BallPosition {
        position: (ground_to_field * ball.position),
        velocity: Vector::zeros(),
        last_seen: cycle_start_time,
    })
}

#[allow(clippy::too_many_arguments)]
fn generate_role(
    replacement_keeper_priority: Option<usize>,
    striker_priority: Option<usize>,
    goal_keeper_number: Option<usize>,
    striker_jersey_number: usize,
    own_jersey_number: usize,
    offense_optional_roles: &[Role],
    defense_optional_roles: &[Role],
    number_of_defensive_players: usize,
) -> Role {
    if replacement_keeper_priority.is_some() || striker_priority.is_some() {
        pick_role_with_penalties(
            replacement_keeper_priority.unwrap(),
            striker_priority.unwrap(),
            striker_jersey_number,
            own_jersey_number,
            offense_optional_roles,
            defense_optional_roles,
            number_of_defensive_players,
        )
    } else if Some(own_jersey_number) == goal_keeper_number {
        return Role::Keeper;
    } else {
        Role::Striker // This case only happens if we don't have a game controller state
    }
}

fn pick_role_with_penalties(
    // penalties: &Players<Option<Penalty>>,
    replacement_keeper_priority: usize,
    striker_priority: usize,
    striker_jersey_number: usize,
    own_jersey_number: usize,
    offense_optional_roles: &[Role],
    defense_optional_roles: &[Role],
    number_of_defensive_players: usize,
) -> Role {
    // let mut role_assignment: Players<Option<Role>> = Players {
    //     one: None,
    //     two: None,
    //     three: None,
    //     four: None,
    //     five: None,
    //     six: None,
    //     seven: None,
    // };

    // role_assignment[striker_player_number] = Some(Role::Striker);
    // let mut unassigned_robots = 6;

    // unassigned_robots -= penalties
    //     .iter()
    //     .filter(|(_player, &penalty)| penalty.is_some())
    //     .count();

    // if unassigned_robots > 0 {
    //     unassigned_robots =
    //         assign_keeper_or_replacement_keeper(unassigned_robots, penalties, &mut role_assignment);
    // }
    if replacement_keeper_priority == 0
        || (replacement_keeper_priority == 1 && own_jersey_number > striker_jersey_number)
    {
        return Role::ReplacementKeeper;
    }

    // for &optional_role in offense_optional_roles.iter().take(unassigned_robots) {
    //     if needs_assignment(PlayerNumber::Two, penalties, &role_assignment) {
    //         role_assignment[PlayerNumber::Two] = Some(optional_role);
    //     } else if needs_assignment(PlayerNumber::Three, penalties, &role_assignment) {
    //         role_assignment[PlayerNumber::Three] = Some(optional_role);
    //     } else if needs_assignment(PlayerNumber::Four, penalties, &role_assignment) {
    //         role_assignment[PlayerNumber::Four] = Some(optional_role);
    //     } else if needs_assignment(PlayerNumber::Five, penalties, &role_assignment) {
    //         role_assignment[PlayerNumber::Five] = Some(optional_role);
    //     } else if needs_assignment(PlayerNumber::Six, penalties, &role_assignment) {
    //         role_assignment[PlayerNumber::Six] = Some(optional_role);
    //     } else if needs_assignment(PlayerNumber::Seven, penalties, &role_assignment) {
    //         role_assignment[PlayerNumber::Seven] = Some(optional_role);
    //     }
    // }

    // role_assignment[own_player_number].unwrap_or_default()
    if replacement_keeper_priority <= number_of_defensive_players {
        defense_optional_roles
            .get(replacement_keeper_priority)
            .copied()
            .unwrap_or_default()
    } else {
        match own_jersey_number.cmp(&striker_jersey_number) {
            Ordering::Greater => offense_optional_roles
                .get(striker_priority)
                .copied()
                .unwrap_or_default(),
            Ordering::Equal => Role::Striker,
            Ordering::Less => offense_optional_roles
                .get(striker_priority - 1)
                .copied()
                .unwrap_or_default(),
        }
    }
}

// fn needs_assignment(
//     player_number: PlayerNumber,
//     penalties: &Players<Option<Penalty>>,
//     role_assignment: &Players<Option<Role>>,
// ) -> bool {
//     role_assignment[player_number].is_none() && penalties[player_number].is_none()
// }

// fn assign_keeper_or_replacement_keeper(
//     unassigned_robots: usize,
//     penalties: &Players<Option<Penalty>>,
//     role_assignment: &mut Players<Option<Role>>,
// ) -> usize {
//     if needs_assignment(PlayerNumber::One, penalties, role_assignment) {
//         role_assignment[PlayerNumber::One] = Some(Role::Keeper);
//         return unassigned_robots - 1;
//     }

//     if needs_assignment(PlayerNumber::Two, penalties, role_assignment) {
//         role_assignment[PlayerNumber::Two] = Some(Role::ReplacementKeeper);
//         return unassigned_robots - 1;
//     } else if needs_assignment(PlayerNumber::Three, penalties, role_assignment) {
//         role_assignment[PlayerNumber::Three] = Some(Role::ReplacementKeeper);
//         return unassigned_robots - 1;
//     } else if needs_assignment(PlayerNumber::Four, penalties, role_assignment) {
//         role_assignment[PlayerNumber::Four] = Some(Role::ReplacementKeeper);
//         return unassigned_robots - 1;
//     } else if needs_assignment(PlayerNumber::Five, penalties, role_assignment) {
//         role_assignment[PlayerNumber::Five] = Some(Role::ReplacementKeeper);
//         return unassigned_robots - 1;
//     } else if needs_assignment(PlayerNumber::Six, penalties, role_assignment) {
//         role_assignment[PlayerNumber::Six] = Some(Role::ReplacementKeeper);
//         return unassigned_robots - 1;
//     } else if needs_assignment(PlayerNumber::Seven, penalties, role_assignment) {
//         role_assignment[PlayerNumber::Seven] = Some(Role::ReplacementKeeper);
//         return unassigned_robots - 1;
//     }

//     unassigned_robots
// }
