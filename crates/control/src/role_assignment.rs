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
    last_received_striker_message: Option<SystemTime>,
    last_system_time_transmitted_game_controller_return_message: Option<SystemTime>,
    last_transmitted_spl_message: Option<SystemTime>,
    role: Role,
    last_time_player_was_penalized: Players<Option<SystemTime>>,
}

#[context]
pub struct CreationContext {
    optional_roles: Parameter<Vec<Role>, "behavior.optional_roles">,
    player_number: Parameter<PlayerNumber, "player_number">,
}

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
    time_to_reach_kick_position: Input<Option<Duration>, "time_to_reach_kick_position?">,
    team_ball: Input<Option<BallPosition<Field>>, "team_ball?">,

    field_dimensions: Parameter<FieldDimensions, "field_dimensions">,
    forced_role: Parameter<Option<Role>, "role_assignment.forced_role?">,
    keeper_replacementkeeper_switch_time:
        Parameter<Duration, "role_assignment.keeper_replacementkeeper_switch_time">,
    striker_trusts_team_ball: Parameter<Duration, "role_assignment.striker_trusts_team_ball">,
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
    pub fn new(context: CreationContext) -> Result<Self> {
        #[allow(clippy::get_first)]
        let role = match context.player_number {
            PlayerNumber::One => Some(Role::Keeper),
            PlayerNumber::Two => context.optional_roles.get(0).copied(),
            PlayerNumber::Three => context.optional_roles.get(1).copied(),
            PlayerNumber::Four => context.optional_roles.get(2).copied(),
            PlayerNumber::Five => context.optional_roles.get(3).copied(),
            PlayerNumber::Six => context.optional_roles.get(4).copied(),
            PlayerNumber::Seven => Some(Role::Striker),
        }
        .unwrap_or(Role::Striker);
        Ok(Self {
            last_received_striker_message: None,
            last_system_time_transmitted_game_controller_return_message: None,
            last_transmitted_spl_message: None,
            role,
            last_time_player_was_penalized: Players::new(None),
        })
    }

    pub fn cycle(
        &mut self,
        mut context: CycleContext<impl NetworkInterface>,
    ) -> Result<MainOutputs> {
        let cycle_start_time = context.cycle_time.start_time;
        let primary_state = *context.primary_state;

        self.try_sending_game_controller_return_message(&context)?;

        let role_from_state_machine =
            self.role_from_state_machine(&context, cycle_start_time, self.role);

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

        let mut new_role = [
            context.forced_role.copied(),
            self.role_for_ready_and_set(&context),
            role_for_penalty_shootout(context.filtered_game_controller_state),
            keep_current_role_in_penalty_kick(context.filtered_game_controller_state, self.role),
            keep_current_role_if_not_in_playing(primary_state, self.role),
            keep_current_role_during_free_kicks(context.filtered_game_controller_state, self.role),
            Some(role_from_state_machine),
        ]
        .iter()
        .find_map(|maybe_role| *maybe_role)
        .expect("at least role_from_state_machine should be Some");

        if self.role == Role::ReplacementKeeper && new_role != Role::Striker {
            let lowest_player_number_without_penalty = self
                .last_time_player_was_penalized
                .iter()
                .find_map(|(player_number, penalized_time)| {
                    match penalized_time {
                        None => true,
                        Some(penalized_time) => {
                            let since_last_penalized = cycle_start_time
                                .duration_since(*penalized_time)
                                .expect("time ran backwards");
                            since_last_penalized >= *context.keeper_replacementkeeper_switch_time
                        }
                    }
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
                        self.try_sending_striker_message(&context)?;
                    }
                }
                (_other_role, Role::Striker) => {
                    self.try_sending_striker_message(&context)?;
                }

                (Role::Striker, Role::Loser) => {
                    self.try_sending_loser_message(&context)?;
                }
                _ => {}
            }
        }

        self.role = new_role;

        Ok(MainOutputs {
            role: self.role.into(),
        })
    }

    fn role_for_ready_and_set(
        &mut self,
        context: &CycleContext<'_, impl NetworkInterface>,
    ) -> Option<Role> {
        if *context.primary_state == PrimaryState::Ready
            || *context.primary_state == PrimaryState::Set
        {
            #[allow(clippy::get_first)]
            let mut player_roles = Players {
                one: Some(Role::Keeper),
                two: context.optional_roles.get(0).copied(),
                three: context.optional_roles.get(1).copied(),
                four: context.optional_roles.get(2).copied(),
                five: context.optional_roles.get(3).copied(),
                six: context.optional_roles.get(4).copied(),
                seven: Some(Role::Striker),
            }
            .map(|role| role.unwrap_or(Role::Striker));

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

            self.last_received_striker_message = None;

            return Some(player_roles[*context.player_number]);
        }

        None
    }

    fn role_from_state_machine(
        &mut self,
        context: &CycleContext<'_, impl NetworkInterface>,
        cycle_start_time: SystemTime,
        current_role: Role,
    ) -> Role {
        let spl_striker_message_timeout = match self.last_received_striker_message {
            None => false,
            Some(last_received_spl_striker_message) => {
                if cycle_start_time
                    .duration_since(last_received_spl_striker_message)
                    .expect("time ran backwards")
                    > context.spl_network.spl_striker_message_receive_timeout
                {
                    self.last_received_striker_message = None;
                    true
                } else {
                    false
                }
            }
        };
        let striker_message_timeout_event = spl_striker_message_timeout
            .then_some(Event::Loser)
            .into_iter();

        let messages: Vec<_> = context
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
            .collect();

        let events = striker_message_timeout_event
            .chain(messages)
            // Update the state machine at least once
            .chain([Event::None]);

        let mut new_role = current_role;
        for event in events {
            if let Event::Striker(_) = event {
                self.last_received_striker_message = Some(cycle_start_time)
            }

            new_role = update_role_state_machine(
                new_role,
                context.ball_position.is_some(),
                event,
                context.time_to_reach_kick_position.copied(),
                context.team_ball.copied(),
                cycle_start_time,
                context.filtered_game_controller_state,
                *context.player_number,
                *context.striker_trusts_team_ball,
                context.optional_roles,
            );
        }

        new_role
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
            self.last_transmitted_spl_message,
            context.spl_network.spl_striker_message_send_interval,
        )
    }

    fn is_striker_silence_period_elapsed(
        &self,
        context: &CycleContext<impl NetworkInterface>,
    ) -> bool {
        is_cooldown_elapsed(
            context.cycle_time.start_time,
            self.last_transmitted_spl_message,
            context.spl_network.silence_interval_between_messages,
        )
    }

    fn try_sending_game_controller_return_message(
        &mut self,
        context: &CycleContext<impl NetworkInterface>,
    ) -> Result<()> {
        if !self.is_return_message_cooldown_elapsed(context) {
            return Ok(());
        }
        let Some(address) = context.game_controller_address else {
            return Ok(());
        };
        let ground_to_field = ground_to_field_or_initial_pose(context);

        self.last_system_time_transmitted_game_controller_return_message =
            Some(context.cycle_time.start_time);
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
    ) -> Result<()> {
        if !self.is_striker_silence_period_elapsed(context) {
            return Ok(());
        }
        if !is_enough_message_budget_left(context) {
            return Ok(());
        }

        self.last_transmitted_spl_message = Some(context.cycle_time.start_time);
        self.last_received_striker_message = None;

        let ground_to_field = ground_to_field_or_initial_pose(context);
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
                time_to_reach_kick_position: *context.time_to_reach_kick_position.unwrap(),
            })))
            .wrap_err("failed to write StrikerMessage to hardware")
    }

    fn try_sending_loser_message(
        &mut self,
        context: &CycleContext<impl NetworkInterface>,
    ) -> Result<()> {
        if !is_enough_message_budget_left(context) {
            return Ok(());
        }

        self.last_transmitted_spl_message = Some(context.cycle_time.start_time);
        self.last_received_striker_message = None;

        let ground_to_field = ground_to_field_or_initial_pose(context);
        context
            .hardware
            .write_to_network(OutgoingMessage::Spl(HulkMessage::Loser(LoserMessage {
                player_number: *context.player_number,
                pose: ground_to_field.as_pose(),
            })))
            .wrap_err("failed to write LoserMessage to hardware")
    }
}

fn ground_to_field_or_initial_pose(
    context: &CycleContext<'_, impl NetworkInterface>,
) -> Isometry2<Ground, Field> {
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
        })
}

fn is_allowed_to_send_messages(context: &CycleContext<'_, impl NetworkInterface>) -> bool {
    let is_playing = *context.primary_state == PrimaryState::Playing;
    let is_penalty_kick =
        context
            .filtered_game_controller_state
            .is_some_and(|game_controller_state| {
                matches!(
                    game_controller_state.game_phase,
                    GamePhase::PenaltyShootout { .. }
                ) || matches!(game_controller_state.sub_state, Some(SubState::PenaltyKick))
            });

    is_playing && !is_penalty_kick
}

fn is_cooldown_elapsed(now: SystemTime, last: Option<SystemTime>, cooldown: Duration) -> bool {
    match last {
        None => true,
        Some(last_time) => now.duration_since(last_time).expect("time ran backwards") > cooldown,
    }
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

#[derive(Clone, Copy, Debug)]
enum Event {
    None,
    Striker(StrikerEvent),
    Loser,
}

#[derive(Clone, Copy, Debug)]
struct StrikerEvent {
    player_number: PlayerNumber,
    time_to_reach_kick_position: Duration,
}

#[allow(clippy::too_many_arguments)]
fn update_role_state_machine(
    current_role: Role,
    detected_own_ball: bool,
    event: Event,
    time_to_reach_kick_position: Option<Duration>,
    team_ball: Option<BallPosition<Field>>,
    cycle_start_time: SystemTime,
    filtered_game_controller_state: Option<&FilteredGameControllerState>,
    player_number: PlayerNumber,
    striker_trusts_team_ball: Duration,
    optional_roles: &[Role],
) -> Role {
    let striker_trusts_team_ball = |team_ball: BallPosition<Field>| {
        cycle_start_time
            .duration_since(team_ball.last_seen)
            .expect("time ran backwards")
            <= striker_trusts_team_ball
    };

    match (current_role, detected_own_ball, event) {
        // Striker lost Ball
        (Role::Striker, false, Event::None | Event::Loser) => match team_ball {
            Some(team_ball) if striker_trusts_team_ball(team_ball) => Role::Striker,
            _ => match player_number{
                PlayerNumber::One => Role::Keeper,
                _ => Role::Loser,
            }
        },

        (_other_role, _, Event::Striker(striker_event)) => claim_striker_or_other_role(
            striker_event,
            time_to_reach_kick_position,
            player_number,
            filtered_game_controller_state,
            optional_roles,
        ),

        // Striker remains Striker
        (Role::Striker, true, Event::None) => Role::Striker,

        // Edge-case, another Striker became Loser, so we claim striker since we see a ball
        (Role::Striker, true, Event::Loser) => Role::Striker,

        // Loser remains Loser
        (Role::Loser, false, Event::None) => Role::Loser,
        (Role::Loser, false, Event::Loser) => Role::Loser,

        // Loser found ball and becomes Striker
        (Role::Loser, true, Event::None) => Role::Striker,

        // Edge-case, Loser found Ball at the same time as receiving a loser message
        (Role::Loser, true, Event::Loser) => Role::Striker,

        // Searcher remains Searcher
        (Role::Searcher, false, Event::None) |
        // Edge-case, a striker (which should not exist) lost the ball
        (Role::Searcher, false, Event::Loser) => {
            pick_keeper_or_searcher(player_number, filtered_game_controller_state)
        },

        // Searcher found ball and becomes Striker
        (Role::Searcher, true, Event::None) => Role::Striker,

        // Searcher found Ball at the same time as receiving a message
        (Role::Searcher, true, Event::Loser) => Role::Striker,

        // Remain in other_role
        (other_role, false, Event::None) => other_role,

        // Either someone found or lost a ball. if found: do I want to claim striker ?
        (other_role, false, Event::Loser) => {
            if other_role == Role::Keeper || other_role == Role::ReplacementKeeper {
                other_role
            } else {
                Role::Searcher
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

fn keep_current_role_if_not_in_playing(
    primary_state: PrimaryState,
    current_role: Role,
) -> Option<Role> {
    if primary_state != PrimaryState::Playing {
        return Some(current_role);
    }
    None
}

fn role_for_penalty_shootout(
    filtered_game_controller_state: Option<&FilteredGameControllerState>,
) -> Option<Role> {
    if let Some(game_controller_state) = filtered_game_controller_state {
        if let GamePhase::PenaltyShootout { kicking_team } = game_controller_state.game_phase {
            return Some(match kicking_team {
                Team::Hulks => Role::Striker,
                Team::Opponent => Role::Keeper,
            });
        };
    }
    None
}

fn keep_current_role_in_penalty_kick(
    filtered_game_controller_state: Option<&FilteredGameControllerState>,
    current_role: Role,
) -> Option<Role> {
    if let Some(game_controller_state) = filtered_game_controller_state {
        if let Some(SubState::PenaltyKick) = game_controller_state.sub_state {
            return Some(current_role);
        }
    }
    None
}

fn keep_current_role_during_free_kicks(
    filtered_game_controller_state: Option<&FilteredGameControllerState>,
    current_role: Role,
) -> Option<Role> {
    if let Some(FilteredGameControllerState {
        sub_state: Some(SubState::KickIn | SubState::PushingFreeKick),
        kicking_team: None,
        ..
    }) = filtered_game_controller_state
    {
        if current_role != Role::Searcher && current_role != Role::Striker {
            return Some(current_role);
        }
    }
    None
}

fn claim_striker_or_other_role(
    striker_event: StrikerEvent,
    time_to_reach_kick_position: Option<Duration>,
    player_number: PlayerNumber,
    filtered_game_controller_state: Option<&FilteredGameControllerState>,
    optional_roles: &[Role],
) -> Role {
    let shorter_time_to_reach =
        time_to_reach_kick_position.unwrap() < striker_event.time_to_reach_kick_position;
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
    let mut unassigned_players: VecDeque<_> = penalties
        .iter()
        .filter_map(|(player_number, penalty)| {
            (player_number != striker_player_number && penalty.is_none()).then_some(player_number)
        })
        .collect();

    let mut role_assignment: Players<Option<Role>> = Players::new(None);
    role_assignment[striker_player_number] = Some(Role::Striker);
    if let Some(keeper) = unassigned_players.pop_front() {
        role_assignment[keeper] = Some(match keeper {
            PlayerNumber::One => Role::Keeper,
            _ => Role::ReplacementKeeper,
        })
    }

    for (player_number, &optional_role) in unassigned_players.into_iter().zip(optional_roles) {
        role_assignment[player_number] = Some(optional_role)
    }

    role_assignment[own_player_number].unwrap_or(Role::Striker)
}

fn pick_keeper_or_searcher(
    own_player_number: PlayerNumber,
    filtered_game_controller_state: Option<&FilteredGameControllerState>,
) -> Role {
    let Some(filtered_game_controller_state) = filtered_game_controller_state else {
        // This case only happens if we don't have a game controller state
        return Role::Searcher;
    };

    let mut unassigned_players: VecDeque<_> = filtered_game_controller_state
        .penalties
        .iter()
        .filter_map(|(player_number, penalty)| penalty.is_none().then_some(player_number))
        .collect();

    if unassigned_players.pop_front() == Some(own_player_number) {
        return match own_player_number {
            PlayerNumber::One => Role::Keeper,
            _ => Role::ReplacementKeeper,
        };
    }

    Role::Searcher
}

#[cfg(test)]
mod test {
    use proptest::prelude::*;

    use super::*;

    proptest! {
        #[allow(clippy::too_many_arguments)]
        #[test]
        fn process_role_state_machine_should_be_idempotent_with_event_none(
            initial_role in prop_oneof![
                Just(Role::DefenderLeft),
                Just(Role::DefenderRight),
                Just(Role::Keeper),
                Just(Role::Loser),
                Just(Role::MidfielderLeft),
                Just(Role::MidfielderRight),
                Just(Role::ReplacementKeeper),
                Just(Role::Searcher),
                Just(Role::Striker),
                Just(Role::StrikerSupporter),
            ],
            detected_own_ball: bool,
            event in Just(Event::None),
            time_to_reach_kick_position in prop_oneof![Just(None), Just(Some(Duration::ZERO)), Just(Some(Duration::from_secs(10_000)))],
            team_ball in prop_oneof![
                Just(None),
                Just(Some(BallPosition::<Field>{ last_seen: SystemTime::UNIX_EPOCH, position: Default::default(), velocity: Default::default() })),
                Just(Some(BallPosition{ last_seen: SystemTime::UNIX_EPOCH + Duration::from_secs(10), position: Default::default(), velocity: Default::default() }))
            ],
            cycle_start_time in prop_oneof![Just(SystemTime::UNIX_EPOCH + Duration::from_secs(11))],
            filtered_game_controller_state in prop_oneof![Just(None), Just(Some(FilteredGameControllerState{game_phase: GamePhase::PenaltyShootout{kicking_team: Team::Hulks}, ..Default::default()}))],
            player_number in Just(PlayerNumber::Five),
            striker_trusts_team_ball_duration in  Just(Duration::from_secs(5)),
            optional_roles in Just(&[Role::DefenderLeft, Role::StrikerSupporter])
        ) {
            let filtered_game_controller_state: Option<FilteredGameControllerState> = filtered_game_controller_state;
            let new_role = update_role_state_machine(
                initial_role,
                detected_own_ball,
                event,
                time_to_reach_kick_position,
                team_ball,
                cycle_start_time,
                filtered_game_controller_state.as_ref(),
                player_number,
                striker_trusts_team_ball_duration,
                optional_roles,
            );
            let third_role = update_role_state_machine(
                new_role,
                detected_own_ball,
                Event::None,
                time_to_reach_kick_position,
                team_ball,
                cycle_start_time,
                filtered_game_controller_state.as_ref(),
                player_number,
                striker_trusts_team_ball_duration,
                optional_roles,
            );
            assert_eq!(new_role, third_role);
        }
    }
}
