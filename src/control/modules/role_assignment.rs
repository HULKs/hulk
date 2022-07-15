use std::{
    sync::Arc,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use anyhow::Result;
use log::debug;
use module_derive::{module, require_some};
use nalgebra::{Isometry2, Point2};
use spl_network::{GameControllerReturnMessage, Penalty, PlayerNumber, SplMessage};
use tokio::sync::{
    mpsc::{unbounded_channel, UnboundedReceiver, UnboundedSender},
    Mutex,
};
use types::{
    BallPosition, FallState, FieldDimensions, GameControllerState, Players, PrimaryState, Role,
    SensorData,
};

use crate::{framework::configuration::SplNetwork, spl_network::MessageReceivers};

pub struct RoleAssignment {
    game_controller_return_message_receiver:
        Arc<Mutex<UnboundedReceiver<GameControllerReturnMessage>>>,
    game_controller_return_message_sender: UnboundedSender<GameControllerReturnMessage>,
    last_received_spl_striker_message: Option<SystemTime>,
    last_transmitted_game_controller_return_message: Option<SystemTime>,
    last_transmitted_spl_striker_message: Option<SystemTime>,
    spl_message_receiver: Arc<Mutex<UnboundedReceiver<SplMessage>>>,
    spl_message_sender: UnboundedSender<SplMessage>,
    role: Role,
    role_initialized: bool,
    team_ball: Option<BallPosition>,
}

#[module(control)]
#[input(path = ball_position, data_type = BallPosition)]
#[input(path = fall_state, data_type = FallState)]
#[input(path = game_controller_state, data_type = GameControllerState)]
#[input(path = robot_to_field, data_type = Isometry2<f32>)]
#[input(path = sensor_data, data_type = SensorData)]
#[input(path = primary_state, data_type = PrimaryState)]
#[perception_input(path = spl_message, data_type = SplMessage, cycler = spl_network)]
#[parameter(path = control.role_assignment.forced_role, data_type = Option<Role>)]
#[parameter(path = field_dimensions, data_type = FieldDimensions)]
#[parameter(path = player_number, data_type = PlayerNumber)]
#[parameter(path = spl_network, data_type = SplNetwork)]
#[main_output(data_type = BallPosition, name = team_ball)]
#[main_output(data_type = MessageReceivers)]
#[main_output(data_type = Vec<Point2<f32>>, name = network_robot_obstacles)]
#[main_output(data_type = Role)]
impl RoleAssignment {}

impl RoleAssignment {
    fn new(_context: NewContext) -> anyhow::Result<Self> {
        let (game_controller_return_message_sender, game_controller_return_message_receiver) =
            unbounded_channel();
        let (spl_message_sender, spl_message_receiver) = unbounded_channel();
        Ok(Self {
            game_controller_return_message_receiver: Arc::new(Mutex::new(
                game_controller_return_message_receiver,
            )),
            game_controller_return_message_sender,
            last_received_spl_striker_message: None,
            last_transmitted_game_controller_return_message: None,
            last_transmitted_spl_striker_message: None,
            spl_message_receiver: Arc::new(Mutex::new(spl_message_receiver)),
            spl_message_sender,
            role: Role::default(),
            role_initialized: false,
            team_ball: None,
        })
    }

    fn cycle(&mut self, context: CycleContext) -> Result<MainOutputs> {
        let cycle_start_time = require_some!(context.sensor_data).cycle_info.start_time;
        let ball = context.ball_position;
        let fall_state = require_some!(context.fall_state);
        let robot_to_field = context.robot_to_field.unwrap_or_default();
        let primary_state = *require_some!(context.primary_state);
        let mut role = self.role;

        if !self.role_initialized
            || primary_state == PrimaryState::Ready
            || primary_state == PrimaryState::Set
        {
            role = match context.player_number {
                PlayerNumber::One => Role::Keeper,
                PlayerNumber::Two => Role::DefenderRight,
                PlayerNumber::Three => Role::StrikerSupporter,
                PlayerNumber::Four => Role::DefenderLeft,
                PlayerNumber::Five => Role::Striker,
            };
            self.role_initialized = true;
            self.last_received_spl_striker_message = Some(cycle_start_time);
            self.team_ball = None;
        }

        let send_game_controller_return_message = self
            .last_transmitted_game_controller_return_message
            .is_none()
            || cycle_start_time.duration_since(
                self.last_transmitted_game_controller_return_message
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
            self.last_transmitted_game_controller_return_message = Some(cycle_start_time);
            self.game_controller_return_message_sender
                .send(GameControllerReturnMessage {
                    player_number: *context.player_number,
                    fallen: matches!(fall_state, FallState::Fallen { .. }),
                    robot_to_field,
                    ball_position: seen_ball_to_network_ball_position(ball, cycle_start_time),
                })?;
        }

        let mut team_ball = self.team_ball;

        if spl_striker_message_timeout {
            match role {
                Role::Keeper => {}
                Role::ReplacementKeeper => {}
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

        let mut spl_messages = context
            .spl_message
            .persistent
            .values()
            .flatten()
            .copied()
            .flatten()
            .peekable();

        let mut network_robot_obstacles = vec![];

        if spl_messages.peek().is_none() {
            (role, send_spl_striker_message, team_ball) = process_role_state_machine(
                role,
                &robot_to_field,
                ball,
                primary_state,
                None,
                send_spl_striker_message,
                team_ball,
                cycle_start_time,
                context.game_controller_state,
                *context.player_number,
                context.spl_network.striker_trusts_team_ball,
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
                    &robot_to_field,
                    ball,
                    primary_state,
                    Some(spl_message),
                    send_spl_striker_message,
                    team_ball,
                    cycle_start_time,
                    context.game_controller_state,
                    *context.player_number,
                    context.spl_network.striker_trusts_team_ball,
                );
            }
        }

        if send_spl_striker_message
            && primary_state == PrimaryState::Playing
            && silence_interval_has_passed
        {
            self.last_transmitted_spl_striker_message = Some(cycle_start_time);
            self.last_received_spl_striker_message = Some(cycle_start_time);
            if let Some(game_controller_state) = *context.game_controller_state {
                if game_controller_state.remaining_amount_of_messages
                    > context
                        .spl_network
                        .remaining_amount_of_messages_to_stop_sending
                {
                    if ball.is_none() && team_ball.is_some() {
                        self.spl_message_sender.send(SplMessage {
                            player_number: *context.player_number,
                            fallen: matches!(fall_state, FallState::Fallen { .. }),
                            robot_to_field,
                            ball_position: team_ball_to_network_ball_position(
                                &team_ball,
                                &robot_to_field,
                                cycle_start_time,
                            ),
                        })?;
                    } else {
                        self.spl_message_sender.send(SplMessage {
                            player_number: *context.player_number,
                            fallen: matches!(fall_state, FallState::Fallen { .. }),
                            robot_to_field,
                            ball_position: seen_ball_to_network_ball_position(
                                ball,
                                cycle_start_time,
                            ),
                        })?;
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
            role: Some(self.role),
            team_ball: self.team_ball,
            message_receivers: Some(MessageReceivers {
                game_controller_return_message_receiver: self
                    .game_controller_return_message_receiver
                    .clone(),
                spl_message_receiver: self.spl_message_receiver.clone(),
            }),
            network_robot_obstacles: Some(network_robot_obstacles),
        })
    }
}

#[allow(clippy::too_many_arguments)]
fn process_role_state_machine(
    current_role: Role,
    current_pose: &Isometry2<f32>,
    detected_own_ball: &Option<BallPosition>,
    primary_state: PrimaryState,
    incoming_message: Option<&SplMessage>,
    send_spl_striker_message: bool,
    team_ball: Option<BallPosition>,
    cycle_start_time: SystemTime,
    game_controller_state: &Option<GameControllerState>,
    player_number: PlayerNumber,
    striker_trusts_team_ball: Duration,
) -> (Role, bool, Option<BallPosition>) {
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
            None => {
                print_case("1", ".1", player_number, cycle_start_time);
                (Role::Loser, true, None)
            }
            Some(team_ball) => {
                if cycle_start_time
                    .duration_since(team_ball.last_seen)
                    .unwrap()
                    > striker_trusts_team_ball
                {
                    print_case("1", ".2.1", player_number, cycle_start_time);
                    (Role::Loser, true, None)
                } else {
                    print_case("1", ".2.2", player_number, cycle_start_time);
                    (Role::Striker, send_spl_striker_message, Some(team_ball))
                }
            }
        },

        // Striker maybe lost Ball but got a message (edge-case)
        (Role::Striker, None, Some(spl_message)) => match &spl_message.ball_position {
            None => {
                // another Striker became Loser
                match team_ball {
                    None => {
                        print_case("2", ".1.1", player_number, cycle_start_time);
                        (Role::Loser, true, None)
                    }
                    Some(team_ball) => {
                        if cycle_start_time
                            .duration_since(team_ball.last_seen)
                            .unwrap()
                            > striker_trusts_team_ball
                        {
                            print_case("2", ".1.2.1", player_number, cycle_start_time);
                            (Role::Loser, true, None)
                        } else {
                            print_case("2", ".1.2.2", player_number, cycle_start_time);
                            (Role::Striker, send_spl_striker_message, Some(team_ball))
                        }
                    }
                }
            }
            Some(spl_message_ball_position) => decide_if_claiming_striker_or_other_role(
                "2.2",
                current_pose,
                spl_message,
                spl_message_ball_position,
                player_number,
                cycle_start_time,
                game_controller_state,
            ),
        },

        //Striker remains Striker, sends message after timeout
        (Role::Striker, Some(..), None) => {
            print_case("3", "", player_number, cycle_start_time);
            (Role::Striker, send_spl_striker_message, team_ball)
        }

        // Striker got a message (either another Player claims Stiker role or Edge-case of a second Striker)
        (Role::Striker, Some(..), Some(spl_message)) => match &spl_message.ball_position {
            None => {
                // another Striker became Loser, so we claim striker since we see a ball
                print_case("4", ".1", player_number, cycle_start_time);
                (
                    Role::Striker,
                    true,
                    team_ball_from_seen_ball(detected_own_ball, current_pose, cycle_start_time),
                )
            }
            Some(spl_message_ball_position) => decide_if_claiming_striker_or_other_role(
                "4",
                current_pose,
                spl_message,
                spl_message_ball_position,
                player_number,
                cycle_start_time,
                game_controller_state,
            ),
        },

        //Loser remains Loser
        (Role::Loser, None, None) => {
            print_case("5", "", player_number, cycle_start_time);
            (Role::Loser, false, team_ball)
        }

        (Role::Loser, None, Some(spl_message)) => match &spl_message.ball_position {
            None => {
                print_case("6", ".1", player_number, cycle_start_time);
                (Role::Loser, false, None)
            } //edge-case, a striker (which should not exist) lost the ball
            Some(spl_message_ball_position) => decide_if_claiming_striker_or_other_role(
                "6",
                current_pose,
                spl_message,
                spl_message_ball_position,
                player_number,
                cycle_start_time,
                game_controller_state,
            ),
        },

        //Loser found ball and becomes Striker
        (Role::Loser, Some(..), None) => {
            print_case("7", "", player_number, cycle_start_time);
            (
                Role::Striker,
                true,
                team_ball_from_seen_ball(detected_own_ball, current_pose, cycle_start_time),
            )
        }

        // Edge-case, Loser found Ball at the same time as receiving a message
        (Role::Loser, Some(..), Some(spl_message)) => match &spl_message.ball_position {
            None => {
                // another Striker became Loser, so we claim striker since we see a ball
                print_case("8", ".1", player_number, cycle_start_time);
                (
                    Role::Striker,
                    true,
                    team_ball_from_seen_ball(detected_own_ball, current_pose, cycle_start_time),
                )
            }
            Some(spl_message_ball_position) => decide_if_claiming_striker_or_other_role(
                "8",
                current_pose,
                spl_message,
                spl_message_ball_position,
                player_number,
                cycle_start_time,
                game_controller_state,
            ),
        },

        //Searcher remains Searcher
        (Role::Searcher, None, None) => {
            print_case("9", "", player_number, cycle_start_time);
            (Role::Searcher, false, team_ball)
        }

        (Role::Searcher, None, Some(spl_message)) => match &spl_message.ball_position {
            None => {
                print_case("10", ".1", player_number, cycle_start_time);
                (Role::Searcher, false, team_ball)
            } //edge-case, a striker (which should not exist) lost the ball
            Some(spl_message_ball_position) => decide_if_claiming_striker_or_other_role(
                "10",
                current_pose,
                spl_message,
                spl_message_ball_position,
                player_number,
                cycle_start_time,
                game_controller_state,
            ),
        },

        //Searcher found ball and becomes Striker
        (Role::Searcher, Some(..), None) => {
            print_case("11", "", player_number, cycle_start_time);
            (
                Role::Striker,
                true,
                team_ball_from_seen_ball(detected_own_ball, current_pose, cycle_start_time),
            )
        }

        // TODO: Searcher found Ball at the same time as receiving a message
        (Role::Searcher, Some(..), Some(spl_message)) => match &spl_message.ball_position {
            None => {
                print_case("12", ".1", player_number, cycle_start_time);
                (
                    Role::Striker,
                    true,
                    team_ball_from_seen_ball(detected_own_ball, current_pose, cycle_start_time),
                )
            }
            Some(spl_message_ball_position) => decide_if_claiming_striker_or_other_role(
                "12",
                current_pose,
                spl_message,
                spl_message_ball_position,
                player_number,
                cycle_start_time,
                game_controller_state,
            ),
        },

        // remain in other_role
        (other_role, None, None) => {
            print_case("13", "", player_number, cycle_start_time);
            (other_role, false, team_ball)
        }

        // Either someone found or lost a ball. if found: do I want to claim striker ?
        (other_role, None, Some(spl_message)) => match &spl_message.ball_position {
            None => {
                if other_role != Role::Keeper && other_role != Role::ReplacementKeeper {
                    print_case("14", ".1.1", player_number, cycle_start_time);
                    (Role::Searcher, false, None)
                } else {
                    print_case("14", ".1.2", player_number, cycle_start_time);
                    (other_role, false, None)
                }
            }
            Some(spl_message_ball_position) => decide_if_claiming_striker_or_other_role(
                "14",
                current_pose,
                spl_message,
                spl_message_ball_position,
                player_number,
                cycle_start_time,
                game_controller_state,
            ),
        },

        // Claim Striker if team-ball position is None
        (other_role, Some(..), None) => match team_ball {
            None => {
                print_case("15", ".1", player_number, cycle_start_time);
                (
                    Role::Striker,
                    true,
                    team_ball_from_seen_ball(detected_own_ball, current_pose, cycle_start_time),
                )
            }
            Some(..) => {
                print_case("15", ".2", player_number, cycle_start_time);
                (
                    other_role,
                    false,
                    team_ball_from_seen_ball(detected_own_ball, current_pose, cycle_start_time),
                )
            }
        },

        // if message is Ball-Lost => Striker, claim Striker ? design-decision: which ball to trust ?
        (_other_role, Some(..), Some(spl_message)) => match &spl_message.ball_position {
            None => {
                print_case("16", ".1", player_number, cycle_start_time);
                (
                    Role::Striker,
                    true,
                    team_ball_from_seen_ball(detected_own_ball, current_pose, cycle_start_time),
                )
            }
            Some(spl_message_ball_position) => decide_if_claiming_striker_or_other_role(
                "16",
                current_pose,
                spl_message,
                spl_message_ball_position,
                player_number,
                cycle_start_time,
                game_controller_state,
            ),
        },
    }
}

fn decide_if_claiming_striker_or_other_role(
    case: &str,
    current_pose: &Isometry2<f32>,
    spl_message: &SplMessage,
    spl_message_ball_position: &spl_network::BallPosition,
    player_number: PlayerNumber,
    cycle_start_time: SystemTime,
    game_controller_state: &Option<GameControllerState>,
) -> (Role, bool, Option<BallPosition>) {
    if am_better_striker(
        current_pose,
        &spl_message.robot_to_field,
        spl_message_ball_position,
    ) {
        print_case(case, ".2.1", player_number, cycle_start_time);
        (
            Role::Striker,
            true,
            team_ball_from_spl_message(cycle_start_time, spl_message),
        )
    } else {
        print_case(case, ".2.2", player_number, cycle_start_time);
        (
            generate_role(
                player_number,
                game_controller_state,
                spl_message.player_number,
            ),
            false,
            team_ball_from_spl_message(cycle_start_time, spl_message),
        )
    }
}

fn seen_ball_to_network_ball_position(
    ball: &Option<BallPosition>,
    cycle_start_time: SystemTime,
) -> Option<spl_network::BallPosition> {
    ball.map(|ball| spl_network::BallPosition {
        age: cycle_start_time.duration_since(ball.last_seen).unwrap(),
        relative_position: ball.position,
    })
}

fn team_ball_to_network_ball_position(
    team_ball: &Option<BallPosition>,
    robot_to_field: &Isometry2<f32>,
    cycle_start_time: SystemTime,
) -> Option<spl_network::BallPosition> {
    team_ball.map(|team_ball| spl_network::BallPosition {
        age: cycle_start_time
            .duration_since(team_ball.last_seen)
            .unwrap(),
        relative_position: robot_to_field.inverse() * team_ball.position,
    })
}

fn team_ball_from_spl_message(
    cycle_start_time: SystemTime,
    spl_message: &SplMessage,
) -> Option<BallPosition> {
    spl_message
        .ball_position
        .as_ref()
        .map(|ball_position| BallPosition {
            position: spl_message.robot_to_field * ball_position.relative_position,
            last_seen: cycle_start_time - ball_position.age,
        })
}

fn team_ball_from_seen_ball(
    ball: &Option<BallPosition>,
    current_pose: &Isometry2<f32>,
    cycle_start_time: SystemTime,
) -> Option<BallPosition> {
    ball.as_ref().map(|ball| BallPosition {
        position: (current_pose * ball.position),
        last_seen: cycle_start_time,
    })
}

fn am_better_striker(
    current_pose: &Isometry2<f32>,
    origin_pose: &Isometry2<f32>,
    spl_message_ball_position: &spl_network::BallPosition,
) -> bool {
    (current_pose.inverse() * origin_pose * spl_message_ball_position.relative_position)
        .coords
        .norm()
        < spl_message_ball_position.relative_position.coords.norm()
}

fn generate_role(
    own_player_number: PlayerNumber,
    game_controller_state: &Option<GameControllerState>,
    striker_player_number: PlayerNumber,
) -> Role {
    if let Some(state) = game_controller_state {
        pick_role_with_penalties(own_player_number, &state.penalties, striker_player_number)
    } else {
        Role::Striker // This case only happens if we don't have a game controller state
    }
}

fn pick_role_with_penalties(
    own_player_number: PlayerNumber,
    penalties: &Players<Option<Penalty>>,
    striker_player_number: PlayerNumber,
) -> Role {
    let mut role_assignment: Players<Option<Role>> = Players {
        one: None,
        two: None,
        three: None,
        four: None,
        five: None,
    };
    role_assignment[striker_player_number] = Some(Role::Striker);
    let mut keeper_or_replacement_keeper_found = false;
    let mut unassigned_robots = 4;

    if penalties[PlayerNumber::One].is_some() {
        unassigned_robots -= 1;
    }
    if penalties[PlayerNumber::Two].is_some() {
        unassigned_robots -= 1;
    }
    if penalties[PlayerNumber::Three].is_some() {
        unassigned_robots -= 1;
    }
    if penalties[PlayerNumber::Four].is_some() {
        unassigned_robots -= 1;
    }
    if penalties[PlayerNumber::Five].is_some() {
        unassigned_robots -= 1;
    }

    if unassigned_robots > 0
        && role_assignment[PlayerNumber::One].is_none()
        && penalties[PlayerNumber::One].is_none()
    {
        role_assignment[PlayerNumber::One] = Some(Role::Keeper);
        keeper_or_replacement_keeper_found = true;
        unassigned_robots -= 1;
    }

    if !keeper_or_replacement_keeper_found
        && role_assignment[PlayerNumber::Two].is_none()
        && penalties[PlayerNumber::Two].is_none()
    {
        role_assignment[PlayerNumber::Two] = Some(Role::ReplacementKeeper);
        unassigned_robots -= 1;
    } else if !keeper_or_replacement_keeper_found
        && role_assignment[PlayerNumber::Three].is_none()
        && penalties[PlayerNumber::Three].is_none()
    {
        role_assignment[PlayerNumber::Three] = Some(Role::ReplacementKeeper);
        unassigned_robots -= 1;
    } else if !keeper_or_replacement_keeper_found
        && role_assignment[PlayerNumber::Four].is_none()
        && penalties[PlayerNumber::Four].is_none()
    {
        role_assignment[PlayerNumber::Four] = Some(Role::ReplacementKeeper);
        unassigned_robots -= 1;
    } else if !keeper_or_replacement_keeper_found
        && role_assignment[PlayerNumber::Five].is_none()
        && penalties[PlayerNumber::Five].is_none()
    {
        role_assignment[PlayerNumber::Five] = Some(Role::ReplacementKeeper);
        unassigned_robots -= 1;
    }

    if unassigned_robots > 0
        && role_assignment[PlayerNumber::Two].is_none()
        && penalties[PlayerNumber::Two].is_none()
    {
        role_assignment[PlayerNumber::Two] = Some(Role::DefenderRight);
        unassigned_robots -= 1;
    } else if unassigned_robots > 0
        && role_assignment[PlayerNumber::Three].is_none()
        && penalties[PlayerNumber::Three].is_none()
    {
        role_assignment[PlayerNumber::Three] = Some(Role::DefenderRight);
        unassigned_robots -= 1;
    } else if unassigned_robots > 0
        && role_assignment[PlayerNumber::Four].is_none()
        && penalties[PlayerNumber::Four].is_none()
    {
        role_assignment[PlayerNumber::Four] = Some(Role::DefenderRight);
        unassigned_robots -= 1;
    } else if unassigned_robots > 0
        && role_assignment[PlayerNumber::Five].is_none()
        && penalties[PlayerNumber::Five].is_none()
    {
        role_assignment[PlayerNumber::Five] = Some(Role::DefenderRight);
        unassigned_robots -= 1;
    }

    if unassigned_robots > 0
        && role_assignment[PlayerNumber::Two].is_none()
        && penalties[PlayerNumber::Two].is_none()
    {
        role_assignment[PlayerNumber::Two] = Some(Role::StrikerSupporter);
        unassigned_robots -= 1;
    } else if unassigned_robots > 0
        && role_assignment[PlayerNumber::Three].is_none()
        && penalties[PlayerNumber::Three].is_none()
    {
        role_assignment[PlayerNumber::Three] = Some(Role::StrikerSupporter);
        unassigned_robots -= 1;
    } else if unassigned_robots > 0
        && role_assignment[PlayerNumber::Four].is_none()
        && penalties[PlayerNumber::Four].is_none()
    {
        role_assignment[PlayerNumber::Four] = Some(Role::StrikerSupporter);
        unassigned_robots -= 1;
    } else if unassigned_robots > 0
        && role_assignment[PlayerNumber::Five].is_none()
        && penalties[PlayerNumber::Five].is_none()
    {
        role_assignment[PlayerNumber::Five] = Some(Role::StrikerSupporter);
        unassigned_robots -= 1;
    }

    if unassigned_robots > 0
        && role_assignment[PlayerNumber::Two].is_none()
        && penalties[PlayerNumber::Two].is_none()
    {
        role_assignment[PlayerNumber::Two] = Some(Role::DefenderLeft);
    } else if unassigned_robots > 0
        && role_assignment[PlayerNumber::Three].is_none()
        && penalties[PlayerNumber::Three].is_none()
    {
        role_assignment[PlayerNumber::Three] = Some(Role::DefenderLeft);
    } else if unassigned_robots > 0
        && role_assignment[PlayerNumber::Four].is_none()
        && penalties[PlayerNumber::Four].is_none()
    {
        role_assignment[PlayerNumber::Four] = Some(Role::DefenderLeft);
    } else if unassigned_robots > 0
        && role_assignment[PlayerNumber::Five].is_none()
        && penalties[PlayerNumber::Five].is_none()
    {
        role_assignment[PlayerNumber::Five] = Some(Role::DefenderLeft);
    }

    role_assignment[own_player_number].unwrap()
}

fn print_case(
    case: &str,
    sub_case: &str,
    player_numer: PlayerNumber,
    cycle_start_time: SystemTime,
) {
    let show_all = false;
    let force_quiet = true; // Switch for debugging
    let mut full_case = String::from(case);
    full_case.push_str(sub_case);
    // Spam avoidance
    if !force_quiet
        && (show_all
            || (case != "14"
                && case != "16"
                && case != "15"
                && full_case != "13"
                && full_case != "3"
                && full_case != "14.2.2"
                && full_case != "5"
                && full_case != "9"
                && full_case != "1.2.2"
                && full_case != "10.2.2"
                && full_case != "7"
                && full_case != "1.2.1"
                && full_case != "10.2.1"
                && full_case != "2.2.2"
                && full_case != "2.2.1"
                && full_case != "4.2.2"
                && full_case != "4.2.1"
                && full_case != "11"
                && full_case != "6.2.1"
                && full_case != "6.2.2"))
    {
        debug!(
            "[{}] Player {:?} in case {}{}",
            cycle_start_time
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs_f64(),
            player_numer,
            case,
            sub_case,
        )
    }
}
