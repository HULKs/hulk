use std::time::{Duration, SystemTime};

use color_eyre::{eyre::WrapErr, Result};
use context_attribute::context;
use framework::{MainOutput, PerceptionInput};
use hardware::NetworkInterface;
use nalgebra::{Isometry2, Point2, Vector2};
use spl_network_messages::{
    GameControllerReturnMessage, GamePhase, HulkMessage, Penalty, PlayerNumber, Team,
};
use types::{
    configuration::SplNetwork,
    messages::{IncomingMessage, OutgoingMessage},
    BallPosition, CycleTime, FallState, FieldDimensions, GameControllerState, InitialPose, Players,
    PrimaryState, Role,
};

use crate::localization::generate_initial_pose;

pub struct RoleAssignment {
    last_received_spl_striker_message: Option<SystemTime>,
    last_system_time_transmitted_game_controller_return_message: Option<SystemTime>,
    last_transmitted_spl_striker_message: Option<SystemTime>,
    role: Role,
    role_initialized: bool,
    team_ball: Option<BallPosition>,
}

#[context]
pub struct CreationContext {
    pub forced_role: Parameter<Option<Role>, "role_assignment.forced_role?">,
    pub player_number: Parameter<PlayerNumber, "player_number">,
    pub spl_network: Parameter<SplNetwork, "spl_network">,
}

#[context]
pub struct CycleContext {
    pub ball_position: Input<Option<BallPosition>, "ball_position?">,
    pub fall_state: Input<FallState, "fall_state">,
    pub game_controller_state: Input<Option<GameControllerState>, "game_controller_state?">,
    pub primary_state: Input<PrimaryState, "primary_state">,
    pub robot_to_field: Input<Option<Isometry2<f32>>, "robot_to_field?">,
    pub cycle_time: Input<CycleTime, "cycle_time">,
    pub network_message: PerceptionInput<IncomingMessage, "SplNetwork", "message">,
    pub time_to_reach_kick_position: PersistentState<Duration, "time_to_reach_kick_position">,

    pub field_dimensions: Parameter<FieldDimensions, "field_dimensions">,
    pub forced_role: Parameter<Option<Role>, "role_assignment.forced_role?">,
    pub initial_poses: Parameter<Players<InitialPose>, "localization.initial_poses">,
    pub optional_roles: Parameter<Vec<Role>, "behavior.optional_roles">,
    pub player_number: Parameter<PlayerNumber, "player_number">,
    pub spl_network: Parameter<SplNetwork, "spl_network">,

    pub hardware: HardwareInterface,
}

#[context]
#[derive(Default)]
pub struct MainOutputs {
    pub team_ball: MainOutput<Option<BallPosition>>,
    pub network_robot_obstacles: MainOutput<Vec<Point2<f32>>>,
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
        })
    }

    pub fn cycle(&mut self, context: CycleContext<impl NetworkInterface>) -> Result<MainOutputs> {
        let cycle_start_time = context.cycle_time.start_time;
        let primary_state = *context.primary_state;
        let mut role = self.role;

        let robot_to_field =
            context
                .robot_to_field
                .copied()
                .unwrap_or_else(|| match context.primary_state {
                    PrimaryState::Initial => generate_initial_pose(
                        &context.initial_poses[*context.player_number],
                        context.field_dimensions,
                    ),
                    _ => Default::default(),
                });

        if !self.role_initialized
            || primary_state == PrimaryState::Ready
            || primary_state == PrimaryState::Set
        {
            role = match context.player_number {
                PlayerNumber::One => Role::Keeper,
                PlayerNumber::Two => context.optional_roles.get(0).copied().unwrap_or_default(),
                PlayerNumber::Three => context.optional_roles.get(1).copied().unwrap_or_default(),
                PlayerNumber::Four => context.optional_roles.get(2).copied().unwrap_or_default(),
                PlayerNumber::Five => context.optional_roles.get(3).copied().unwrap_or_default(),
                PlayerNumber::Six => context.optional_roles.get(4).copied().unwrap_or_default(),
                PlayerNumber::Seven => Role::Striker,
            };
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
            context
                .hardware
                .write_to_network(OutgoingMessage::GameController(
                    GameControllerReturnMessage {
                        player_number: *context.player_number,
                        fallen: matches!(context.fall_state, FallState::Fallen { .. }),
                        robot_to_field,
                        ball_position: seen_ball_to_network_ball_position(
                            context.ball_position,
                            cycle_start_time,
                        ),
                    },
                ))
                .wrap_err("failed to write GameControllerReturnMessage to hardware")?;
        }

        let mut team_ball = self.team_ball;

        if spl_striker_message_timeout {
            match role {
                Role::Keeper => {
                    team_ball = None;
                }
                Role::ReplacementKeeper => {
                    team_ball = None;
                }
                Role::Striker => {
                    send_spl_striker_message = true;
                    team_ball = None;
                    role = Role::Loser;
                }
                _ => {
                    send_spl_striker_message = false;
                    team_ball = None;
                    role = Role::Searcher
                }
            }
        }

        let mut network_robot_obstacles = vec![];
        let mut spl_messages = context
            .network_message
            .persistent
            .values()
            .flatten()
            .filter_map(|message| match message {
                IncomingMessage::GameController(_) => None,
                IncomingMessage::Spl(message) => Some(message),
            })
            .peekable();
        if spl_messages.peek().is_none() {
            (role, send_spl_striker_message, team_ball) = process_role_state_machine(
                role,
                robot_to_field,
                context.ball_position,
                primary_state,
                None,
                Some(*context.time_to_reach_kick_position),
                send_spl_striker_message,
                team_ball,
                cycle_start_time,
                context.game_controller_state,
                *context.player_number,
                context.spl_network.striker_trusts_team_ball,
                context.optional_roles,
            );
        } else {
            for spl_message in spl_messages {
                self.last_received_spl_striker_message = Some(cycle_start_time);
                let sender_position =
                    (robot_to_field.inverse() * spl_message.robot_to_field) * Point2::origin();
                if spl_message.player_number != *context.player_number {
                    network_robot_obstacles.push(sender_position);
                }
                (role, send_spl_striker_message, team_ball) = process_role_state_machine(
                    role,
                    robot_to_field,
                    context.ball_position,
                    primary_state,
                    Some(spl_message),
                    Some(*context.time_to_reach_kick_position),
                    send_spl_striker_message,
                    team_ball,
                    cycle_start_time,
                    context.game_controller_state,
                    *context.player_number,
                    context.spl_network.striker_trusts_team_ball,
                    context.optional_roles,
                );
            }
        }

        if send_spl_striker_message
            && primary_state == PrimaryState::Playing
            && silence_interval_has_passed
        {
            self.last_transmitted_spl_striker_message = Some(cycle_start_time);
            self.last_received_spl_striker_message = Some(cycle_start_time);
            if let Some(game_controller_state) = context.game_controller_state {
                if game_controller_state.remaining_amount_of_messages
                    > context
                        .spl_network
                        .remaining_amount_of_messages_to_stop_sending
                {
                    if context.ball_position.is_none() && team_ball.is_some() {
                        context
                            .hardware
                            .write_to_network(OutgoingMessage::Spl(HulkMessage {
                                player_number: *context.player_number,
                                fallen: matches!(context.fall_state, FallState::Fallen { .. }),
                                robot_to_field,
                                ball_position: team_ball_to_network_ball_position(
                                    team_ball,
                                    robot_to_field,
                                    cycle_start_time,
                                ),
                                time_to_reach_kick_position: Some(
                                    *context.time_to_reach_kick_position,
                                ),
                            }))?;
                    } else {
                        context
                            .hardware
                            .write_to_network(OutgoingMessage::Spl(HulkMessage {
                                player_number: *context.player_number,
                                fallen: matches!(context.fall_state, FallState::Fallen { .. }),
                                robot_to_field,
                                ball_position: seen_ball_to_network_ball_position(
                                    context.ball_position,
                                    cycle_start_time,
                                ),
                                time_to_reach_kick_position: Some(
                                    *context.time_to_reach_kick_position,
                                ),
                            }))?;
                    }
                }
            }
        }

        if let Some(forced_role) = context.forced_role {
            self.role = *forced_role;
        } else {
            self.role = role;
        }
        self.team_ball = team_ball;

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
    current_pose: Isometry2<f32>,
    detected_own_ball: Option<&BallPosition>,
    primary_state: PrimaryState,
    incoming_message: Option<&HulkMessage>,
    time_to_reach_kick_position: Option<Duration>,
    send_spl_striker_message: bool,
    team_ball: Option<BallPosition>,
    cycle_start_time: SystemTime,
    game_controller_state: Option<&GameControllerState>,
    player_number: PlayerNumber,
    striker_trusts_team_ball: Duration,
    optional_roles: &[Role],
) -> (Role, bool, Option<BallPosition>) {
    if let Some(game_controller_state) = game_controller_state {
        match game_controller_state.game_phase {
            GamePhase::PenaltyShootout {
                kicking_team: Team::Hulks,
            } => return (Role::Striker, false, None),
            GamePhase::PenaltyShootout {
                kicking_team: Team::Opponent,
            } => return (Role::Keeper, false, None),
            _ => {}
        }
    }

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

    if let Some(message) = incoming_message {
        if message.player_number == player_number {
            return (current_role, false, team_ball);
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
                player_number,
                cycle_start_time,
                game_controller_state,
                optional_roles,
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
                player_number,
                cycle_start_time,
                game_controller_state,
                optional_roles,
            ),
        },

        //Loser remains Loser
        (Role::Loser, None, None) => (Role::Loser, false, team_ball),

        (Role::Loser, None, Some(spl_message)) => match &spl_message.ball_position {
            None => (Role::Loser, false, None), //edge-case, a striker (which should not exist) lost the ball
            _ => decide_if_claiming_striker_or_other_role(
                spl_message,
                time_to_reach_kick_position,
                player_number,
                cycle_start_time,
                game_controller_state,
                optional_roles,
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
                player_number,
                cycle_start_time,
                game_controller_state,
                optional_roles,
            ),
        },

        //Searcher remains Searcher
        (Role::Searcher, None, None) => (Role::Searcher, false, team_ball),

        (Role::Searcher, None, Some(spl_message)) => match &spl_message.ball_position {
            None => (Role::Searcher, false, team_ball), //edge-case, a striker (which should not exist) lost the ball
            _ => decide_if_claiming_striker_or_other_role(
                spl_message,
                time_to_reach_kick_position,
                player_number,
                cycle_start_time,
                game_controller_state,
                optional_roles,
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
                player_number,
                cycle_start_time,
                game_controller_state,
                optional_roles,
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
                player_number,
                cycle_start_time,
                game_controller_state,
                optional_roles,
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
                player_number,
                cycle_start_time,
                game_controller_state,
                optional_roles,
            ),
        },
    }
}

fn decide_if_claiming_striker_or_other_role(
    spl_message: &HulkMessage,
    time_to_reach_kick_position: Option<Duration>,
    player_number: PlayerNumber,
    cycle_start_time: SystemTime,
    game_controller_state: Option<&GameControllerState>,
    optional_roles: &[Role],
) -> (Role, bool, Option<BallPosition>) {
    if time_to_reach_kick_position < spl_message.time_to_reach_kick_position {
        (
            Role::Striker,
            true,
            team_ball_from_spl_message(cycle_start_time, spl_message),
        )
    } else {
        (
            generate_role(
                player_number,
                game_controller_state,
                spl_message.player_number,
                optional_roles,
            ),
            false,
            team_ball_from_spl_message(cycle_start_time, spl_message),
        )
    }
}

fn seen_ball_to_network_ball_position(
    ball: Option<&BallPosition>,
    cycle_start_time: SystemTime,
) -> Option<spl_network_messages::BallPosition> {
    ball.map(|ball| spl_network_messages::BallPosition {
        age: cycle_start_time.duration_since(ball.last_seen).unwrap(),
        relative_position: ball.position,
    })
}

fn team_ball_to_network_ball_position(
    team_ball: Option<BallPosition>,
    robot_to_field: Isometry2<f32>,
    cycle_start_time: SystemTime,
) -> Option<spl_network_messages::BallPosition> {
    team_ball.map(|team_ball| spl_network_messages::BallPosition {
        age: cycle_start_time
            .duration_since(team_ball.last_seen)
            .unwrap(),
        relative_position: robot_to_field.inverse() * team_ball.position,
    })
}

fn team_ball_from_spl_message(
    cycle_start_time: SystemTime,
    spl_message: &HulkMessage,
) -> Option<BallPosition> {
    spl_message
        .ball_position
        .as_ref()
        .map(|ball_position| BallPosition {
            position: spl_message.robot_to_field * ball_position.relative_position,
            velocity: Vector2::zeros(),
            last_seen: cycle_start_time - ball_position.age,
        })
}

fn team_ball_from_seen_ball(
    ball: Option<&BallPosition>,
    current_pose: Isometry2<f32>,
    cycle_start_time: SystemTime,
) -> Option<BallPosition> {
    ball.as_ref().map(|ball| BallPosition {
        position: (current_pose * ball.position),
        velocity: Vector2::zeros(),
        last_seen: cycle_start_time,
    })
}

fn generate_role(
    own_player_number: PlayerNumber,
    game_controller_state: Option<&GameControllerState>,
    striker_player_number: PlayerNumber,
    optional_roles: &[Role],
) -> Role {
    if let Some(state) = game_controller_state {
        pick_role_with_penalties(
            own_player_number,
            &state.penalties,
            striker_player_number,
            optional_roles,
        )
    } else {
        Role::Striker // This case only happens if we don't have a game controller state
    }
}

fn pick_role_with_penalties(
    own_player_number: PlayerNumber,
    penalties: &Players<Option<Penalty>>,
    striker_player_number: PlayerNumber,
    optional_roles: &[Role],
) -> Role {
    let mut role_assignment: Players<Option<Role>> = Players {
        one: None,
        two: None,
        three: None,
        four: None,
        five: None,
        six: None,
        seven: None,
    };

    role_assignment[striker_player_number] = Some(Role::Striker);
    let mut unassigned_robots = 6;

    unassigned_robots -= penalties
        .iter()
        .filter(|(_player, &penalty)| penalty.is_some())
        .count();

    if unassigned_robots > 0 {
        unassigned_robots =
            assign_keeper_or_replacement_keeper(unassigned_robots, penalties, &mut role_assignment);
    }

    for &optional_role in optional_roles.iter().take(unassigned_robots) {
        if needs_assignment(PlayerNumber::Two, penalties, &role_assignment) {
            role_assignment[PlayerNumber::Two] = Some(optional_role);
        } else if needs_assignment(PlayerNumber::Three, penalties, &role_assignment) {
            role_assignment[PlayerNumber::Three] = Some(optional_role);
        } else if needs_assignment(PlayerNumber::Four, penalties, &role_assignment) {
            role_assignment[PlayerNumber::Four] = Some(optional_role);
        } else if needs_assignment(PlayerNumber::Five, penalties, &role_assignment) {
            role_assignment[PlayerNumber::Five] = Some(optional_role);
        } else if needs_assignment(PlayerNumber::Six, penalties, &role_assignment) {
            role_assignment[PlayerNumber::Six] = Some(optional_role);
        } else if needs_assignment(PlayerNumber::Seven, penalties, &role_assignment) {
            role_assignment[PlayerNumber::Seven] = Some(optional_role);
        }
    }

    role_assignment[own_player_number].unwrap_or_default()
}

fn needs_assignment(
    player_number: PlayerNumber,
    penalties: &Players<Option<Penalty>>,
    role_assignment: &Players<Option<Role>>,
) -> bool {
    role_assignment[player_number].is_none() && penalties[player_number].is_none()
}

fn assign_keeper_or_replacement_keeper(
    unassigned_robots: usize,
    penalties: &Players<Option<Penalty>>,
    role_assignment: &mut Players<Option<Role>>,
) -> usize {
    if needs_assignment(PlayerNumber::One, penalties, role_assignment) {
        role_assignment[PlayerNumber::One] = Some(Role::Keeper);
        return unassigned_robots - 1;
    }

    if needs_assignment(PlayerNumber::Two, penalties, role_assignment) {
        role_assignment[PlayerNumber::Two] = Some(Role::ReplacementKeeper);
        return unassigned_robots - 1;
    } else if needs_assignment(PlayerNumber::Three, penalties, role_assignment) {
        role_assignment[PlayerNumber::Three] = Some(Role::ReplacementKeeper);
        return unassigned_robots - 1;
    } else if needs_assignment(PlayerNumber::Four, penalties, role_assignment) {
        role_assignment[PlayerNumber::Four] = Some(Role::ReplacementKeeper);
        return unassigned_robots - 1;
    } else if needs_assignment(PlayerNumber::Five, penalties, role_assignment) {
        role_assignment[PlayerNumber::Five] = Some(Role::ReplacementKeeper);
        return unassigned_robots - 1;
    } else if needs_assignment(PlayerNumber::Six, penalties, role_assignment) {
        role_assignment[PlayerNumber::Six] = Some(Role::ReplacementKeeper);
        return unassigned_robots - 1;
    } else if needs_assignment(PlayerNumber::Seven, penalties, role_assignment) {
        role_assignment[PlayerNumber::Seven] = Some(Role::ReplacementKeeper);
        return unassigned_robots - 1;
    }

    unassigned_robots
}
