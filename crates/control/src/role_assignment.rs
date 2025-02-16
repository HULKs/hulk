use std::{
    collections::VecDeque,
    net::SocketAddr,
    time::{Duration, SystemTime},
};

use color_eyre::{
    eyre::{OptionExt, WrapErr},
    Result,
};
use serde::{Deserialize, Serialize};

use context_attribute::context;
use coordinate_systems::{Field, Ground};
use framework::{AdditionalOutput, MainOutput, PerceptionInput};
use hardware::NetworkInterface;
use linear_algebra::Isometry2;
use spl_network_messages::{
    GameControllerReturnMessage, GamePhase, HulkMessage, LoserMessage, Penalty, PlayerNumber,
    StrikerMessage, SubState, Team,
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
    players::Players,
    primary_state::PrimaryState,
    roles::Role,
};

use crate::localization::generate_initial_pose;

#[derive(Deserialize, Serialize)]
pub struct RoleAssignment {
    last_received_spl_striker_message: Option<SystemTime>,
    last_system_time_transmitted_game_controller_return_message: Option<SystemTime>,
    last_transmitted_striker_message: Option<SystemTime>,
    role: Role,
    role_initialized: bool,
    last_time_player_was_penalized: Players<Option<SystemTime>>,
}

#[context]
pub struct CreationContext {}

#[context]
pub struct CycleContext {
    ball_position: Input<Option<BallPosition<Ground>>, "ball_position?">,
    fall_state: Input<FallState, "fall_state">,
    filtered_game_controller_state:
        Input<Option<FilteredGameControllerState>, "filtered_game_controller_state?">,
    primary_state: Input<PrimaryState, "primary_state">,
    ground_to_field: Input<Option<Isometry2<Ground, Field>>, "ground_to_field?">,
    cycle_time: Input<CycleTime, "cycle_time">,
    network_message: PerceptionInput<Option<IncomingMessage>, "SplNetwork", "filtered_message?">,
    game_controller_address: Input<Option<SocketAddr>, "game_controller_address?">,
    time_to_reach_kick_position: Input<Duration, "time_to_reach_kick_position">,
    team_ball: Input<Option<BallPosition<Field>>, "team_ball?">,

    field_dimensions: Parameter<FieldDimensions, "field_dimensions">,
    forced_role: Parameter<Option<Role>, "role_assignment.forced_role?">,
    keeper_replacementkeeper_switch_time:
        Parameter<Duration, "role_assignment.keeper_replacementkeeper_switch_time">,
    initial_poses: Parameter<Players<InitialPose>, "localization.initial_poses">,
    optional_roles: Parameter<Vec<Role>, "behavior.optional_roles">,
    player_number: Parameter<PlayerNumber, "player_number">,
    spl_network: Parameter<SplNetworkParameters, "spl_network">,

    hardware: HardwareInterface,

    last_time_player_was_penalized:
        AdditionalOutput<Players<Option<SystemTime>>, "last_time_player_penalized">,
}

#[context]
#[derive(Default)]
pub struct MainOutputs {
    pub role: MainOutput<Role>,
}

impl RoleAssignment {
    pub fn new(_context: CreationContext) -> Result<Self> {
        Ok(Self {
            last_received_spl_striker_message: None,
            last_system_time_transmitted_game_controller_return_message: None,
            last_transmitted_striker_message: None,
            role: Role::Striker,
            role_initialized: false,
            last_time_player_was_penalized: Players::new(None),
        })
    }

    pub fn cycle(
        &mut self,
        mut context: CycleContext<impl NetworkInterface>,
    ) -> Result<MainOutputs> {
        let cycle_start_time = context.cycle_time.start_time;
        let primary_state = *context.primary_state;
        let mut new_role = self.role;

        let ground_to_field =
            context
                .ground_to_field
                .copied()
                .unwrap_or_else(|| match context.primary_state {
                    PrimaryState::Initial => generate_initial_pose(
                        &context.initial_poses[*context.player_number],
                        context.field_dimensions,
                    )
                    .as_transform(),
                    _ => Default::default(),
                });

        if !self.role_initialized
            || primary_state == PrimaryState::Ready
            || primary_state == PrimaryState::Set
        {
            #[allow(clippy::get_first)]
            let mut player_roles = Players {
                one: Role::Keeper,
                two: context.optional_roles.get(0).copied().unwrap_or_default(),
                three: context.optional_roles.get(1).copied().unwrap_or_default(),
                four: context.optional_roles.get(2).copied().unwrap_or_default(),
                five: context.optional_roles.get(3).copied().unwrap_or_default(),
                six: context.optional_roles.get(4).copied().unwrap_or_default(),
                seven: Role::Striker,
            };

            if let Some(game_controller_state) = context.filtered_game_controller_state {
                if let Some(striker) = [
                    PlayerNumber::Seven,
                    PlayerNumber::Six,
                    PlayerNumber::Five,
                    PlayerNumber::Four,
                ]
                .into_iter()
                .find(|player| game_controller_state.penalties[*player].is_none())
                {
                    player_roles[striker] = Role::Striker;
                }
            }
            new_role = player_roles[*context.player_number];

            self.role_initialized = true;
            self.last_received_spl_striker_message = Some(cycle_start_time);
        }

        self.try_sending_game_controller_return_message(&context, ground_to_field)?;

        // TODO: reimplement whatever this did
        // let is_in_penalty_kick = matches!(
        //     context.filtered_game_controller_state,
        //     Some(FilteredGameControllerState {
        //         sub_state: Some(SubState::PenaltyKick),
        //         ..
        //     })
        // );
        //
        // let spl_striker_message_timeout = match self.last_received_spl_striker_message {
        //     None => false,
        //     Some(last_received_spl_striker_message) => {
        //         cycle_start_time.duration_since(last_received_spl_striker_message)?
        //             > context.spl_network.spl_striker_message_receive_timeout
        //     }
        // };
        //
        // if spl_striker_message_timeout && !is_in_penalty_kick {
        //     match new_role {
        //         Role::Keeper | Role::ReplacementKeeper => {}
        //         Role::Striker => {
        //             new_role = Role::Loser;
        //         }
        //         Role::Loser if *context.player_number == PlayerNumber::One => {
        //             new_role = Role::Keeper;
        //         }
        //         _ => new_role = Role::Searcher,
        //     }
        // }

        let events: Vec<_> = context
            .network_message
            .persistent
            .values()
            .flatten()
            .filter_map(|message| match message {
                Some(IncomingMessage::Spl(HulkMessage::Striker(StrikerMessage {
                    player_number,
                    time_to_reach_kick_position,
                    ..
                }))) => Some(Event::Striker(StrikerEvent {
                    player_number: *player_number,
                    time_to_reach_kick_position: *time_to_reach_kick_position,
                })),
                Some(IncomingMessage::Spl(HulkMessage::Loser(..))) => Some(Event::Loser),
                _ => None,
            })
            // Update the state machine at least once
            .chain([Event::None])
            .collect();

        for event in events {
            self.last_received_spl_striker_message = Some(cycle_start_time);

            new_role = process_role_state_machine(
                new_role,
                context.ball_position.is_some(),
                primary_state,
                event,
                Some(*context.time_to_reach_kick_position),
                context.team_ball.copied(),
                cycle_start_time,
                context.filtered_game_controller_state,
                *context.player_number,
                context.spl_network.striker_trusts_team_ball,
                context.optional_roles,
            );
        }

        if self.role == Role::ReplacementKeeper && new_role != Role::Striker {
            let lowest_player_number_without_penalty = self
                .last_time_player_was_penalized
                .iter()
                .find_map(|(player_number, penalized_time)| {
                    penalized_time
                        .is_none_or(|penalized_time| {
                            let since_last_penalized = cycle_start_time
                                .duration_since(penalized_time)
                                .expect("time ran backwards");
                            since_last_penalized >= *context.keeper_replacementkeeper_switch_time
                        })
                        .then_some(player_number)
                });
            if Some(*context.player_number) == lowest_player_number_without_penalty {
                new_role = Role::ReplacementKeeper;
            }
        }
        context
            .last_time_player_was_penalized
            .fill_if_subscribed(|| self.last_time_player_was_penalized);
        if is_allowed_to_send_messages(&context) {
            match (self.role, new_role) {
                (Role::Striker, Role::Striker) => {
                    if self.is_striker_beacon_cooldown_elapsed(&context) {
                        self.try_sending_striker_message(&context, ground_to_field)?;
                    }
                }
                (_other_role, Role::Striker) => {
                    self.try_sending_striker_message(&context, ground_to_field)?;
                }

                (Role::Striker, Role::Loser) => {
                    self.try_sending_loser_message(&context, ground_to_field)?;
                }
                _ => {}
            }
        }

        if let Some(forced_role) = context.forced_role {
            self.role = *forced_role;
        } else {
            self.role = new_role;
        }

        if let Some(game_controller_state) = context.filtered_game_controller_state {
            for player in self
                .last_time_player_was_penalized
                .clone()
                .iter()
                .map(|(playernumber, ..)| playernumber)
            {
                if game_controller_state.penalties[player].is_some() {
                    self.last_time_player_was_penalized[player] = Some(cycle_start_time);
                }
            }
        }

        Ok(MainOutputs {
            role: self.role.into(),
        })
    }

    fn is_return_message_cooldown_elapsed(
        &self,
        context: &CycleContext<impl NetworkInterface>,
    ) -> bool {
        is_cooldown_elapsed(
            context.cycle_time.start_time,
            self.last_system_time_transmitted_game_controller_return_message,
            context.spl_network.game_controller_return_message_interval,
        )
    }

    fn is_striker_beacon_cooldown_elapsed(
        &self,
        context: &CycleContext<impl NetworkInterface>,
    ) -> bool {
        is_cooldown_elapsed(
            context.cycle_time.start_time,
            self.last_transmitted_striker_message,
            context.spl_network.spl_striker_message_send_interval,
        )
    }

    fn is_striker_silence_period_elapsed(
        &self,
        context: &CycleContext<impl NetworkInterface>,
    ) -> bool {
        is_cooldown_elapsed(
            context.cycle_time.start_time,
            self.last_transmitted_striker_message,
            context.spl_network.silence_interval_between_messages,
        )
    }

    fn try_sending_game_controller_return_message(
        &mut self,
        context: &CycleContext<impl NetworkInterface>,
        ground_to_field: Isometry2<Ground, Field>,
    ) -> Result<()> {
        if !self.is_return_message_cooldown_elapsed(context) {
            return Ok(());
        }
        self.last_system_time_transmitted_game_controller_return_message =
            Some(context.cycle_time.start_time);
        let Some(address) = context.game_controller_address else {
            return Ok(());
        };
        context
            .hardware
            .write_to_network(OutgoingMessage::GameController(
                *address,
                GameControllerReturnMessage {
                    player_number: *context.player_number,
                    fallen: matches!(context.fall_state, FallState::Fallen { .. }),
                    pose: ground_to_field.as_pose(),
                    ball: seen_ball_to_game_controller_ball_position(
                        context.ball_position,
                        context.cycle_time.start_time,
                    ),
                },
            ))
            .wrap_err("failed to write GameControllerReturnMessage to hardware")
    }

    fn try_sending_striker_message(
        &mut self,
        context: &CycleContext<impl NetworkInterface>,
        ground_to_field: Isometry2<Ground, Field>,
    ) -> Result<()> {
        if !self.is_striker_silence_period_elapsed(context) {
            return Ok(());
        }
        if !is_enough_message_budget_left(context) {
            return Ok(());
        }

        self.last_transmitted_striker_message = Some(context.cycle_time.start_time);
        self.last_received_spl_striker_message = Some(context.cycle_time.start_time);

        let pose = ground_to_field.as_pose();
        let team_network_ball = context.team_ball.map(|team_ball| {
            team_ball_to_network_ball_position(*team_ball, context.cycle_time.start_time)
        });
        let own_network_ball = context.ball_position.map(|seen_ball| {
            own_ball_to_hulks_network_ball_position(
                *seen_ball,
                ground_to_field,
                context.cycle_time.start_time,
            )
        });
        let ball_position = own_network_ball
            .or(team_network_ball)
            .ok_or_eyre("we are striker without a ball, this should never happen")?;

        context
            .hardware
            .write_to_network(OutgoingMessage::Spl(HulkMessage::Striker(StrikerMessage {
                player_number: *context.player_number,
                pose,
                ball_position,
                time_to_reach_kick_position: Some(*context.time_to_reach_kick_position),
            })))
            .wrap_err("failed to write StrikerMessage to hardware")
    }

    fn try_sending_loser_message(
        &mut self,
        context: &CycleContext<impl NetworkInterface>,
        ground_to_field: Isometry2<Ground, Field>,
    ) -> Result<()> {
        if !is_enough_message_budget_left(context) {
            return Ok(());
        }

        self.last_transmitted_striker_message = Some(context.cycle_time.start_time);
        self.last_received_spl_striker_message = Some(context.cycle_time.start_time);

        context
            .hardware
            .write_to_network(OutgoingMessage::Spl(HulkMessage::Loser(LoserMessage {
                player_number: *context.player_number,
                pose: ground_to_field.as_pose(),
            })))
            .wrap_err("failed to write LoserMessage to hardware")
    }
}

fn is_allowed_to_send_messages(context: &CycleContext<'_, impl NetworkInterface>) -> bool {
    let is_playing = *context.primary_state == PrimaryState::Playing;
    let not_in_penalty_kick =
        context
            .filtered_game_controller_state
            .is_none_or(|game_controller_state| {
                !matches!(
                    game_controller_state.game_phase,
                    GamePhase::PenaltyShootout { .. }
                ) && !matches!(game_controller_state.sub_state, Some(SubState::PenaltyKick))
            });

    is_playing && not_in_penalty_kick
}

fn is_cooldown_elapsed(now: SystemTime, last: Option<SystemTime>, cooldown: Duration) -> bool {
    last.is_none_or(|last_time| {
        now.duration_since(last_time).expect("time ran backwards") > cooldown
    })
}

fn is_enough_message_budget_left(context: &CycleContext<impl NetworkInterface>) -> bool {
    context
        .filtered_game_controller_state
        .is_some_and(|game_controller_state| {
            game_controller_state.remaining_number_of_messages
                > context
                    .spl_network
                    .remaining_amount_of_messages_to_stop_sending
        })
}

#[derive(Clone, Copy)]
enum Event {
    None,
    Striker(StrikerEvent),
    Loser,
}

#[derive(Clone, Copy)]
struct StrikerEvent {
    player_number: PlayerNumber,
    time_to_reach_kick_position: Option<Duration>,
}

#[allow(clippy::too_many_arguments)]
fn process_role_state_machine(
    current_role: Role,
    detected_own_ball: bool,
    primary_state: PrimaryState,
    event: Event,
    time_to_reach_kick_position: Option<Duration>,
    team_ball: Option<BallPosition<Field>>,
    cycle_start_time: SystemTime,
    filtered_game_controller_state: Option<&FilteredGameControllerState>,
    player_number: PlayerNumber,
    striker_trusts_team_ball_duration: Duration,
    optional_roles: &[Role],
) -> Role {
    if let Some(game_controller_state) = filtered_game_controller_state {
        if let GamePhase::PenaltyShootout { kicking_team } = game_controller_state.game_phase {
            return match kicking_team {
                Team::Hulks => Role::Striker,
                Team::Opponent => Role::Keeper,
            };
        };
        if let Some(SubState::PenaltyKick) = game_controller_state.sub_state {
            return current_role;
        }
    }

    if primary_state != PrimaryState::Playing {
        return current_role;
    }

    let striker_trusts_team_ball = |team_ball: BallPosition<Field>| {
        cycle_start_time
            .duration_since(team_ball.last_seen)
            .expect("time ran backwards")
            <= striker_trusts_team_ball_duration
    };

    match (current_role, detected_own_ball, event) {
        // Striker lost Ball
        (Role::Striker, false, Event::None | Event::Loser) => match team_ball {
            Some(team_ball) if striker_trusts_team_ball(team_ball) => Role::Striker,
            _ => Role::Loser,
        },

        (_other_role, _, Event::Striker(striker_event)) => claim_striker_or_other_role(
            striker_event,
            time_to_reach_kick_position,
            player_number,
            filtered_game_controller_state,
            optional_roles,
        ),

        // Striker remains Striker, sends message after timeout
        (Role::Striker, true, Event::None) => Role::Striker,

        // Edge-case, another Striker became Loser, so we claim striker since we see a ball
        // TODO: On main, this sends a striker message immediately, ignoring the spl_striker_message_send_interval
        //       but not the silence_interval_between_messages.
        //       With the new implementation this is no longer possible since the message sending
        //       is only based on the previous and new roles and does not consider the cause of the
        //       transition.
        //       This was already broken on main since the message would silently be dropped if the
        //       edge case occured within the silence interval but might lead to a faster
        //       striker convergence in this rare circumstance.
        (Role::Striker, true, Event::Loser) => Role::Striker,

        // Loser remains Loser
        (Role::Loser, false, Event::None) => Role::Loser,
        (Role::Loser, false, Event::Loser) => Role::Loser,

        // Loser found ball and becomes Striker
        (Role::Loser, true, Event::None) => Role::Striker,

        // Edge-case, Loser found Ball at the same time as receiving a loser message
        (Role::Loser, true, Event::Loser) => Role::Striker,

        // Searcher remains Searcher
        (Role::Searcher, false, Event::None) => Role::Searcher,

        // Edge-case, a striker (which should not exist) lost the ball
        (Role::Searcher, false, Event::Loser) => Role::Searcher,

        // Searcher found ball and becomes Striker
        (Role::Searcher, true, Event::None) => Role::Striker,

        // Searcher found Ball at the same time as receiving a message
        (Role::Searcher, true, Event::Loser) => Role::Striker,

        // Remain in other_role
        (other_role, false, Event::None) => other_role,

        // Either someone found or lost a ball. if found: do I want to claim striker ?
        (other_role, false, Event::Loser) => {
            if other_role != Role::Keeper && other_role != Role::ReplacementKeeper {
                Role::Searcher
            } else {
                other_role
            }
        }

        // Claim Striker if team-ball is None
        (other_role, true, Event::None) => match team_ball {
            None => Role::Striker,
            Some(..) => other_role,
        },

        // Striker lost ball but we see one, claim striker
        (_other_role, true, Event::Loser) => Role::Striker,
    }
}

fn claim_striker_or_other_role(
    striker_event: StrikerEvent,
    time_to_reach_kick_position: Option<Duration>,
    player_number: PlayerNumber,
    filtered_game_controller_state: Option<&FilteredGameControllerState>,
    optional_roles: &[Role],
) -> Role {
    let shorter_time_to_reach =
        time_to_reach_kick_position < striker_event.time_to_reach_kick_position;
    let time_to_reach_viable =
        time_to_reach_kick_position.is_some_and(|duration| duration < Duration::from_secs(1200));

    if shorter_time_to_reach && time_to_reach_viable {
        return Role::Striker;
    }

    let Some(filtered_game_controller_state) = filtered_game_controller_state else {
        // This case only happens if we don't have a game controller state
        return Role::Striker;
    };

    pick_role_with_penalties(
        player_number,
        &filtered_game_controller_state.penalties,
        striker_event.player_number,
        optional_roles,
    )
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

fn own_ball_to_hulks_network_ball_position(
    ball: BallPosition<Ground>,
    ground_to_field: Isometry2<Ground, Field>,
    cycle_start_time: SystemTime,
) -> spl_network_messages::BallPosition<Field> {
    spl_network_messages::BallPosition {
        age: cycle_start_time.duration_since(ball.last_seen).unwrap(),
        position: ground_to_field * ball.position,
    }
}

fn team_ball_to_network_ball_position(
    team_ball: BallPosition<Field>,
    cycle_start_time: SystemTime,
) -> spl_network_messages::BallPosition<Field> {
    spl_network_messages::BallPosition {
        age: cycle_start_time
            .duration_since(team_ball.last_seen)
            .unwrap(),
        position: team_ball.position,
    }
}

fn pick_role_with_penalties(
    own_player_number: PlayerNumber,
    penalties: &Players<Option<Penalty>>,
    striker_player_number: PlayerNumber,
    optional_roles: &[Role],
) -> Role {
    let mut role_assignment: Players<Option<Role>> = Players::new(None);

    role_assignment[striker_player_number] = Some(Role::Striker);
    let mut unassigned_players: VecDeque<_> = penalties
        .iter()
        .filter_map(|(player_number, penalty)| {
            (player_number != striker_player_number && penalty.is_none()).then_some(player_number)
        })
        .collect();

    if let Some(keeper) = unassigned_players.pop_front() {
        role_assignment[keeper] = Some(match keeper {
            PlayerNumber::One => Role::Keeper,
            _ => Role::ReplacementKeeper,
        })
    }

    for (player_number, &optional_role) in unassigned_players.into_iter().zip(optional_roles) {
        role_assignment[player_number] = Some(optional_role)
    }

    role_assignment[own_player_number].unwrap_or_default()
}

#[cfg(test)]
mod test {
    use test_case::test_matrix;

    use super::*;

    #[allow(clippy::too_many_arguments)]
    #[test_matrix(
        [
            Role::DefenderLeft,
            Role::DefenderRight,
            Role::Keeper,
            Role::Loser,
            Role::MidfielderLeft,
            Role::MidfielderRight,
            Role::ReplacementKeeper,
            Role::Searcher,
            Role::Striker,
            Role::StrikerSupporter,
        ],
        [false, true],
        [PrimaryState::Set, PrimaryState::Playing],
        Event::None,
        [None, Some(Duration::ZERO), Some(Duration::from_secs(10_000))],
        [
            None,
            Some(BallPosition{ last_seen: SystemTime::UNIX_EPOCH, position: Default::default(), velocity: Default::default() }),
            Some(BallPosition{ last_seen: SystemTime::UNIX_EPOCH + Duration::from_secs(10), position: Default::default(), velocity: Default::default() })],
        [SystemTime::UNIX_EPOCH + Duration::from_secs(11)],
        [None, Some(&FilteredGameControllerState{game_phase: GamePhase::PenaltyShootout{kicking_team: Team::Hulks}, ..Default::default()})],
        PlayerNumber::Five,
        Duration::from_secs(5),
        [&[Role::DefenderLeft, Role::StrikerSupporter]]
    )]
    fn process_role_state_machine_should_be_idempotent_with_event_none(
        initial_role: Role,
        detected_own_ball: bool,
        primary_state: PrimaryState,
        event: Event,
        time_to_reach_kick_position: Option<Duration>,
        team_ball: Option<BallPosition<Field>>,
        cycle_start_time: SystemTime,
        filtered_game_controller_state: Option<&FilteredGameControllerState>,
        player_number: PlayerNumber,
        striker_trusts_team_ball_duration: Duration,
        optional_roles: &[Role],
    ) {
        let new_role = process_role_state_machine(
            initial_role,
            detected_own_ball,
            primary_state,
            event,
            time_to_reach_kick_position,
            team_ball,
            cycle_start_time,
            filtered_game_controller_state,
            player_number,
            striker_trusts_team_ball_duration,
            optional_roles,
        );
        let third_role = process_role_state_machine(
            new_role,
            detected_own_ball,
            primary_state,
            Event::None,
            time_to_reach_kick_position,
            team_ball,
            cycle_start_time,
            filtered_game_controller_state,
            player_number,
            striker_trusts_team_ball_duration,
            optional_roles,
        );
        assert_eq!(new_role, third_role);
    }
}
