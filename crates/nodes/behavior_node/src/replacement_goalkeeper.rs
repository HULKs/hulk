// coordinator for decentralized replacement goalkeeper election

use hsl_network_messages::PlayerNumber;
use linear_algebra::point;
use ros_z::time::Time;
use serde::{Deserialize, Serialize}; // was macht das?
use std::time::Duration;
use types::{behavior_tree::Status, players::Players, primary_state::PrimaryState};

use crate::{conditions::is_simple, node::Blackboard, penalty_shootout::is_penalty_shootout};

#[derive(Debug, Clone, Copy)]
pub struct ReplacementGoalkeeperRoleInput {
    pub regular_goalkeeper_is_penalized: bool,
    pub primary_state: PrimaryState,
    pub own_player_number: PlayerNumber,
    pub regular_goalkeeper_number: PlayerNumber,
    pub available_field_players: Players<bool>,
}

pub struct ReplacementGoalkeeperInput {
    pub regular_goalkeeper_is_penalized: bool,
    pub primary_state: PrimaryState,
    pub own_player_number: PlayerNumber,
    pub regular_goalkeeper_number: PlayerNumber,

    pub available_field_players: Players<bool>, // Players on field, abgleitet aus gültigen empfangenen PlayerStates
    pub is_eligible_candidate: bool, // Ich bin verfügbar, meine Pose ist bekannt, ich bin nicht ballzuständig
    pub is_closest_available_field_player_to_own_goal: bool, // unter allen verfügbaren Robotern mit gültiger Pose bin ich der Tornächste. Deren Ballzuständigkeit kenne ich nicht.

    pub lowest_other_claiming_player: Option<PlayerNumber>, // current lowest *other* claim received via StateMessage

    pub now: Time,
}

pub struct ReplacementGoalkeeperParameters {
    // move to parameters
    pub election_window: Duration,
    pub no_claim_timeout: Duration,
    pub backoff_step: Duration,
}

impl Default for ReplacementGoalkeeperParameters {
    fn default() -> Self {
        Self {
            election_window: Duration::from_millis(500),
            no_claim_timeout: Duration::from_secs(2),
            backoff_step: Duration::from_millis(200),
        }
    }
}

// ToDo: Fehlender-GameController Zustand ergibt hier false, wodurch eine bestehende Rolle zurückgesetzt wird. In Zukunft ändern!
/// Refreshes role validity and returns whether keeper selection may proceed.
pub fn refresh_replacement_goalkeeper_election_status(blackboard: &mut Blackboard) -> bool {
    if is_simple(blackboard) || is_penalty_shootout(blackboard) {
        blackboard.replacement_goalkeeper_coordinator.reset();
        return false;
    }

    let world_state = &blackboard.world_state;
    let regular_goalkeeper_number = blackboard.parameters.goalkeeper.player_number;

    let input = ReplacementGoalkeeperRoleInput {
        regular_goalkeeper_is_penalized: world_state
            .filtered_game_controller_state
            .as_ref()
            .is_some_and(|state| state.penalties[regular_goalkeeper_number].is_some()),
        primary_state: world_state.robot.primary_state,
        own_player_number: blackboard.player_number,
        regular_goalkeeper_number: regular_goalkeeper_number,
        available_field_players: calculate_available_field_players(
            blackboard,
            regular_goalkeeper_number,
        ),
    };

    blackboard
        .replacement_goalkeeper_coordinator
        .refresh_role_status(input)
}

pub fn refresh_replacement_goalkeeper_role(blackboard: &mut Blackboard) -> Status {
    refresh_replacement_goalkeeper_election_status(blackboard);
    Status::Success
}
pub(crate) fn build_replacement_goalkeeper_input(
    blackboard: &Blackboard,
    tree_allows_candidacy: bool,
) -> ReplacementGoalkeeperInput {
    let world_state = &blackboard.world_state;
    let own_number = blackboard.player_number;
    let regular_goalkeeper_number = blackboard.parameters.goalkeeper.player_number;
    let available_field_players =
        calculate_available_field_players(blackboard, regular_goalkeeper_number);

    let is_eligible_candidate = tree_allows_candidacy
        && available_field_players[own_number]
        && world_state.robot.ground_to_field.is_some();

    ReplacementGoalkeeperInput {
        regular_goalkeeper_is_penalized: world_state
            .filtered_game_controller_state
            .as_ref()
            .is_some_and(|state| state.penalties[regular_goalkeeper_number].is_some()),
        primary_state: world_state.robot.primary_state,
        own_player_number: own_number,
        regular_goalkeeper_number: regular_goalkeeper_number,
        available_field_players,
        is_eligible_candidate,
        is_closest_available_field_player_to_own_goal:
            is_closest_available_field_player_to_own_goal(blackboard, &available_field_players),
        lowest_other_claiming_player: find_lowest_numbered_available_other_claiming_player(
            blackboard,
            &available_field_players,
            regular_goalkeeper_number,
        ),
        now: world_state.now,
    }
}

/// Returns whether this robot is the closest available field player
/// to its own goal. Equal distances favor the lower player number.
/// Does not check ball responsibility.
pub(crate) fn is_closest_available_field_player_to_own_goal(
    blackboard: &Blackboard,
    available_field_players: &Players<bool>,
) -> bool {
    let world_state = &blackboard.world_state;
    let own_number = blackboard.player_number;
    let regular_goalkeeper_number = blackboard.parameters.goalkeeper.player_number;

    if own_number == regular_goalkeeper_number || !available_field_players[own_number] {
        return false;
    }

    let Some(ground_to_field) = world_state.robot.ground_to_field else {
        return false;
    };

    let own_goal = point!(-blackboard.field_dimensions.length / 2.0, 0.0);

    let own_distance_squared = (ground_to_field.translation() - own_goal).norm_squared();

    for (number, player_state) in world_state.player_states.iter() {
        if number == regular_goalkeeper_number
            || number == own_number
            || !available_field_players[number]
        {
            continue;
        }

        let Some(player_state) = player_state else {
            continue;
        };

        let other_distance_squared = (player_state.inner.pose.position() - own_goal).norm_squared();

        if other_distance_squared < own_distance_squared
            || (other_distance_squared == own_distance_squared && number < own_number)
        {
            return false;
        }
    }

    true
}

pub(crate) fn calculate_available_field_players(
    blackboard: &Blackboard,
    regular_goalkeeper_number: PlayerNumber,
) -> Players<bool> {
    let own_number = blackboard.player_number;
    let mut available = Players::new(false); // five entries with false created

    for (number, player_state) in blackboard.world_state.player_states.iter() {
        if number == regular_goalkeeper_number {
            continue;
        }

        let known_is_penalized = blackboard
            .world_state
            .filtered_game_controller_state
            .as_ref()
            .is_some_and(|state| state.penalties[number].is_some());

        if known_is_penalized {
            continue;
        }

        available[number] = if number == own_number {
            blackboard.world_state.robot.primary_state != PrimaryState::Penalized // Unnötig?
        } else {
            player_state.is_some()
        };
    }

    available
}

/// Returns the lowest-numbered available other field player
/// whose current PlayerState contains a replacement goalkeeper claim.
/// Assumes expired PlayerStates have already been removed.
pub(crate) fn find_lowest_numbered_available_other_claiming_player(
    blackboard: &Blackboard,
    available_field_players: &Players<bool>,
    regular_goalkeeper_number: PlayerNumber,
) -> Option<PlayerNumber> {
    let own_number = blackboard.player_number;

    blackboard
        .world_state
        .player_states
        .iter()
        .filter_map(|(number, player_state)| {
            if number == regular_goalkeeper_number
                || number == own_number
                || !available_field_players[number]
            {
                return None;
            }

            let has_claim = player_state
                .as_ref()
                .is_some_and(|state| state.inner.replacement_goalkeeper_claim);

            has_claim.then_some(number)
        })
        .min()
}

// Election Action, im Playing Tree aufgerufen. Gibt es bessere datei für diese Funktion?
pub fn update_replacement_goalkeeper_election(
    blackboard: &mut Blackboard,
    tree_allows_candidacy: bool,
) -> Status {
    let input = build_replacement_goalkeeper_input(blackboard, tree_allows_candidacy);

    blackboard
        .replacement_goalkeeper_coordinator
        .update(input, &ReplacementGoalkeeperParameters::default());

    Status::Success
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ros_z::Message)]
enum ReplacementGoalkeeperState {
    #[default]
    Inactive,
    Candidate,
    Active,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ros_z::Message)]
pub struct ReplacementGoalkeeperCoordinator {
    state: ReplacementGoalkeeperState,
    known_replacement_goalkeeper: Option<PlayerNumber>,
    election_started_at: Option<Time>,
    replacement_needed_since: Option<Time>,
}

impl ReplacementGoalkeeperCoordinator {
    fn clear_election(&mut self) {
        self.state = ReplacementGoalkeeperState::Inactive;
        self.election_started_at = None;
        self.replacement_needed_since = None;
    }

    fn reset(&mut self) {
        self.clear_election();
        self.known_replacement_goalkeeper = None;
    }

    fn remember_winner(&mut self, winner: PlayerNumber, own_number: PlayerNumber) {
        self.clear_election();
        self.known_replacement_goalkeeper = Some(winner);

        if winner == own_number {
            self.state = ReplacementGoalkeeperState::Active;
        }
    }

    /// Validates the remembered replacement goalkeeper and clears invalid roles
    /// or candidacies. Returns whether keeper selection may proceed.
    /// Does not start or advance an election.
    pub fn refresh_role_status(&mut self, input: ReplacementGoalkeeperRoleInput) -> bool {
        let own_number = input.own_player_number;

        // Warum brauche ich hier den Filter?
        let available_count = input
            .available_field_players
            .iter()
            .filter(|(number, available)| *number != input.regular_goalkeeper_number && **available)
            .count();

        if !input.regular_goalkeeper_is_penalized || available_count <= 1 {
            self.reset();
            return false;
        }

        // Notwendig so elaboriert?
        let own_is_available = own_number != input.regular_goalkeeper_number
            && input.available_field_players[own_number]
            && input.primary_state != PrimaryState::Penalized;

        if let Some(winner) = self.known_replacement_goalkeeper {
            let winner_is_available = winner != input.regular_goalkeeper_number
                && input.available_field_players[winner]
                && (winner != own_number || own_is_available);

            if !winner_is_available {
                self.reset();
            }
        }

        if !own_is_available {
            self.clear_election();
            return false;
        }

        // Eine gültige Besetzung bleibt bestehen, aber muss man das so machen?
        if let Some(winner) = self.known_replacement_goalkeeper {
            self.remember_winner(winner, own_number);
            return false;
        }

        if input.primary_state != PrimaryState::Playing {
            self.clear_election();
        }

        matches!(
            input.primary_state,
            PrimaryState::Ready | PrimaryState::Playing
        )
        // true: Auswahl darf weiter geprüft werden.
        // false: In diesem Update keine Auswahl fortsetzen.
    }

    pub fn update(
        &mut self,
        input: ReplacementGoalkeeperInput,
        parameters: &ReplacementGoalkeeperParameters,
    ) {
        let role_input = ReplacementGoalkeeperRoleInput {
            regular_goalkeeper_is_penalized: input.regular_goalkeeper_is_penalized,
            primary_state: input.primary_state,
            own_player_number: input.own_player_number,
            regular_goalkeeper_number: input.regular_goalkeeper_number,
            available_field_players: input.available_field_players,
        };

        if !self.refresh_role_status(role_input) {
            return;
        }

        let own_number = input.own_player_number;

        match input.primary_state {
            PrimaryState::Ready => {
                self.clear_election();

                let winner = input
                    .available_field_players
                    .iter()
                    .filter_map(|(number, available)| {
                        (number != input.regular_goalkeeper_number && *available).then_some(number)
                    })
                    .min();

                if let Some(winner) = winner {
                    self.remember_winner(winner, own_number);
                }
            }

            PrimaryState::Playing => {
                // Dieser Timer läuft auch ohne eigene Kandidatur.
                let needed_since = *self.replacement_needed_since.get_or_insert(input.now);

                if self.state == ReplacementGoalkeeperState::Candidate
                    && !input.is_eligible_candidate
                {
                    self.state = ReplacementGoalkeeperState::Inactive;
                }

                let other_claim = input.lowest_other_claiming_player;

                // Kleinere verfügbare Nummern beginnen den Fallback früher.
                let backoff_rank = input
                    .available_field_players
                    .iter()
                    .filter(|(number, available)| {
                        **available
                            && *number != input.regular_goalkeeper_number
                            && *number < own_number
                    })
                    .count() as u32;

                let fallback_delay =
                    parameters.no_claim_timeout + parameters.backoff_step * backoff_rank;

                let fallback_due = input.now.duration_since(needed_since) >= fallback_delay;

                let should_start_claim = input.is_eligible_candidate
                    && (input.is_closest_available_field_player_to_own_goal
                        || (other_claim.is_none() && fallback_due));

                if self.state == ReplacementGoalkeeperState::Inactive && should_start_claim {
                    self.state = ReplacementGoalkeeperState::Candidate;
                }

                // Eigenen Claim ohne Netzwerk-Echo einbeziehen.
                let own_claim =
                    (self.state == ReplacementGoalkeeperState::Candidate).then_some(own_number);

                let lowest_claim = other_claim.into_iter().chain(own_claim).min();

                let Some(winner) = lowest_claim else {
                    // Kein Claim: Wahlfenster zurücksetzen.
                    // Der Fallback-Timer läuft weiter.
                    self.election_started_at = None;
                    return;
                };

                // Auch Spieler ohne eigenen Claim beobachten die Wahl.
                let election_start = *self.election_started_at.get_or_insert(input.now);

                let window_elapsed =
                    input.now.duration_since(election_start) >= parameters.election_window;

                if window_elapsed {
                    self.remember_winner(winner, own_number);
                }
            }

            _ => {
                // Insbesondere Set und Stop: keine neue Wahl.
                // Gültige gespeicherte Rollen wurden oben behandelt.
                self.clear_election();
            }
        }
    }

    pub fn is_active(&self) -> bool {
        matches!(self.state, ReplacementGoalkeeperState::Active)
        // kompakter als:
        // match self.state {
        // ReplacementGoalkeeperState::Active => true,
        // _ => false,
        // }
    }

    pub fn claims_role(&self) -> bool {
        matches!(
            self.state,
            ReplacementGoalkeeperState::Candidate | ReplacementGoalkeeperState::Active
        )
    }
}

impl Default for ReplacementGoalkeeperCoordinator {
    fn default() -> Self {
        Self {
            state: ReplacementGoalkeeperState::Inactive,
            known_replacement_goalkeeper: None,
            election_started_at: None,
            replacement_needed_since: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use hsl_network_messages::{GamePhase, HulkMessage, Penalty, Team};
    use linear_algebra::{Isometry2, Pose2};
    use types::{
        field_dimensions::Side,
        filtered_game_controller_state::FilteredGameControllerState,
        filtered_game_state::FilteredGameState,
        messages::OutgoingMessage,
        motion_command::BodyMotion,
        world_state::{BallSource, PlayerState, WorldState},
    };

    use crate::node::LastBall;

    fn blackboard_with_candidate_or_active_replacement(initially_active: bool) -> Blackboard {
        let mut blackboard = goal_distance_blackboard();
        blackboard.parameters.goalkeeper.player_number = PlayerNumber::One;

        let mut game_controller_state = game_controller_state_for_tests();
        game_controller_state.penalties.one = Some(Penalty::PickUp {
            remaining: Duration::from_secs(30),
        });
        blackboard.world_state.filtered_game_controller_state = Some(game_controller_state);

        let parameters = parameters();

        blackboard
            .replacement_goalkeeper_coordinator
            .update(playing_input(Time::zero()), &parameters);

        if initially_active {
            blackboard.world_state.now = Time::from_nanos(500_000_000);

            blackboard
                .replacement_goalkeeper_coordinator
                .update(playing_input(blackboard.world_state.now), &parameters);
        }

        assert!(blackboard.replacement_goalkeeper_coordinator.claims_role());
        assert_eq!(
            blackboard.replacement_goalkeeper_coordinator.is_active(),
            initially_active,
        );

        blackboard
    }

    fn game_controller_state_for_tests() -> FilteredGameControllerState {
        FilteredGameControllerState {
            game_state: FilteredGameState::Playing {
                ball_is_free: true,
                kick_off: false,
            },
            opponent_game_state: FilteredGameState::Playing {
                ball_is_free: true,
                kick_off: false,
            },
            half: hsl_network_messages::Half::First,
            remaining_time_in_half: Duration::ZERO,
            game_phase: GamePhase::Normal,
            kicking_team: None,
            penalties: Players::new(None),
            remaining_number_of_messages: 0,
            sub_state: None,
            global_field_side: types::field_dimensions::GlobalFieldSide::Home,
            new_own_penalties_last_cycle: Default::default(),
            new_opponent_penalties_last_cycle: Default::default(),
        }
    }

    fn goal_distance_blackboard() -> Blackboard {
        let mut blackboard = Blackboard {
            field_dimensions: types::field_dimensions::FieldDimensions::SPL_2025,
            player_number: PlayerNumber::Three,
            head_yaw: 0.0,
            parameters: json5::from_str(include_str!(
                "../../../../etc/parameters/base/behavior_node.json5"
            ))
            .expect("test behavior parameters must be valid"),
            walking_velocity_limits: Default::default(),
            search_cell_size: None,
            world_state: WorldState::default(),
            controller_input: None,
            remote_control_enabled: false,
            replacement_goalkeeper_coordinator: Default::default(),

            path_obstacles_output: Vec::new(),
            time_since_last_switch: Duration::ZERO,
            direction_difference: 0.0,
            voronoi_inputs: Vec::new(),

            ball: None,
            visual_kick_ball_position: None,
            last_ball: None,
            last_close_enough_to_kick: false,
            ball_action: None,
            last_ball_action: None,
            kick_target: None,
            last_kick_target: None,
            last_motion_command: None,
            last_motion_switch_time: Time::zero(),
            last_motion_type: None,
            last_sent_game_controller_return_message_time: None,
            last_sent_hsl_message_time: None,
            last_closest_to_ball: false,
            closest_to_ball_entered_area_since: None,
            closest_to_ball_left_area_since: None,

            is_injected_motion_command: false,
            walk_position: None,
            body_motion: None,
            head_motion: None,
            voronoi_map: None,
        };

        blackboard.parameters.goalkeeper.player_number = PlayerNumber::One;
        blackboard.field_dimensions.length = 10.0;
        blackboard.world_state.robot.primary_state = PrimaryState::Playing;
        blackboard.world_state.robot.ground_to_field = Some(Isometry2::from(point!(-4.0, 0.0)));

        blackboard.world_state.player_states.two = Some(PlayerState {
            pose: Pose2::new(point!(-3.0, 0.0), 0.0),
            head_yaw: 0.0,
            ball_position: None,
            replacement_goalkeeper_claim: false,
        });

        blackboard.world_state.fall_detection = Some(types::fall_detection::FallDetection {
            time: Time::zero(),
            valid_until: Time::from_nanos(1_000_000_000),
            posture: types::fall_detection::Posture::Upright,
        });

        blackboard
    }

    fn parameters() -> ReplacementGoalkeeperParameters {
        ReplacementGoalkeeperParameters {
            election_window: Duration::from_millis(500),
            no_claim_timeout: Duration::from_secs(2),
            backoff_step: Duration::from_millis(200),
        }
    }

    fn playing_input(now: Time) -> ReplacementGoalkeeperInput {
        ReplacementGoalkeeperInput {
            regular_goalkeeper_is_penalized: true,
            primary_state: PrimaryState::Playing,
            own_player_number: PlayerNumber::Three,
            regular_goalkeeper_number: PlayerNumber::One,

            available_field_players: Players {
                one: false,
                two: true,
                three: true,
                four: true,
                five: false,
            },

            is_eligible_candidate: true,
            is_closest_available_field_player_to_own_goal: true,
            lowest_other_claiming_player: None,
            now,
        }
    }

    fn replacement_claim_from_next_state_message(blackboard: &mut Blackboard) -> bool {
        let Some(OutgoingMessage::Hsl(HulkMessage::State(message))) =
            blackboard.try_sending_state_message()
        else {
            panic!("expected an outgoing HSL StateMessage");
        };

        message.replacement_goalkeeper_claim
    }

    fn blackboard_with_claims(claiming_players: &[PlayerNumber]) -> Blackboard {
        let mut blackboard = goal_distance_blackboard();
        blackboard.world_state.player_states = Players::new(None);

        for &number in claiming_players {
            blackboard.world_state.player_states[number] = Some(PlayerState {
                pose: Pose2::new(point!(0.0, 0.0), 0.0),
                head_yaw: 0.0,
                ball_position: None,
                replacement_goalkeeper_claim: true,
            });
        }

        blackboard
    }

    #[test]
    fn becomes_active_only_after_election_window() {
        let mut coordinator = ReplacementGoalkeeperCoordinator::default();
        let parameters = parameters();

        // Bei Wahlbeginn: Candidate, noch kein Gewinner.
        coordinator.update(playing_input(Time::zero()), &parameters);

        assert!(coordinator.claims_role());
        assert!(!coordinator.is_active());
        assert_eq!(coordinator.known_replacement_goalkeeper, None);

        // Nach 499 ms: Das Fenster läuft noch.
        coordinator.update(playing_input(Time::from_nanos(499_000_000)), &parameters);

        assert!(!coordinator.is_active());
        assert_eq!(coordinator.known_replacement_goalkeeper, None);

        // Nach genau 500 ms: Eigener Claim gewinnt ohne Netzwerk-Echo.
        coordinator.update(playing_input(Time::from_nanos(500_000_000)), &parameters);

        assert!(coordinator.is_active());
        assert!(coordinator.claims_role());
        assert_eq!(
            coordinator.known_replacement_goalkeeper,
            Some(PlayerNumber::Three),
        );
    }

    #[test]
    fn remembers_lower_claim_and_ends_own_candidacy() {
        let mut coordinator = ReplacementGoalkeeperCoordinator::default();
        let parameters = parameters();

        // Eigener Spieler 3 und fremder Spieler 2 claimen.
        let mut input = playing_input(Time::zero());
        input.lowest_other_claiming_player = Some(PlayerNumber::Two);
        coordinator.update(input, &parameters);

        assert!(coordinator.claims_role());
        assert!(!coordinator.is_active());

        // Am Wahlende gewinnt Spieler 2.
        let mut input = playing_input(Time::from_nanos(500_000_000));
        input.lowest_other_claiming_player = Some(PlayerNumber::Two);
        coordinator.update(input, &parameters);

        assert!(!coordinator.is_active());
        assert!(!coordinator.claims_role());
        assert_eq!(
            coordinator.known_replacement_goalkeeper,
            Some(PlayerNumber::Two),
        );

        // Ohne frischen Claim bleibt das gespeicherte Ergebnis erhalten.
        coordinator.update(playing_input(Time::from_nanos(600_000_000)), &parameters);

        assert!(!coordinator.claims_role());
        assert_eq!(
            coordinator.known_replacement_goalkeeper,
            Some(PlayerNumber::Two),
        );
    }

    #[test]
    fn clears_replacement_when_regular_goalkeeper_returns() {
        let mut coordinator = ReplacementGoalkeeperCoordinator::default();
        let parameters = parameters();

        // Zunächst wird Spieler 3 gewählt.
        coordinator.update(playing_input(Time::zero()), &parameters);
        coordinator.update(playing_input(Time::from_nanos(500_000_000)), &parameters);

        assert!(coordinator.is_active());

        // Anschließend ist Spieler 1 nicht mehr penalized.
        let mut input = playing_input(Time::from_nanos(600_000_000));
        input.regular_goalkeeper_is_penalized = false;
        coordinator.update(input, &parameters);

        assert!(!coordinator.is_active());
        assert!(!coordinator.claims_role());
        assert_eq!(coordinator.known_replacement_goalkeeper, None);
        assert_eq!(coordinator.election_started_at, None);
        assert_eq!(coordinator.replacement_needed_since, None);
    }

    #[test]
    fn later_lower_claim_does_not_replace_active_keeper() {
        let mut coordinator = ReplacementGoalkeeperCoordinator::default();
        let parameters = parameters();

        // Spieler 3 gewinnt zunächst die Wahl.
        coordinator.update(playing_input(Time::zero()), &parameters);
        coordinator.update(playing_input(Time::from_nanos(500_000_000)), &parameters);

        assert!(coordinator.is_active());

        // Erst danach claimt Spieler 2.
        let mut input = playing_input(Time::from_nanos(600_000_000));
        input.lowest_other_claiming_player = Some(PlayerNumber::Two);
        coordinator.update(input, &parameters);

        assert!(coordinator.is_active());
        assert!(coordinator.claims_role());
        assert_eq!(
            coordinator.known_replacement_goalkeeper,
            Some(PlayerNumber::Three),
        );
    }

    #[test]
    fn own_player_is_available_without_own_player_state() {
        let mut blackboard = goal_distance_blackboard();
        blackboard.player_number = PlayerNumber::Three;
        blackboard.world_state.robot.primary_state = PrimaryState::Playing;
        blackboard.world_state.filtered_game_controller_state =
            Some(game_controller_state_for_tests());
        blackboard.world_state.player_states = Players::new(None);

        let available = calculate_available_field_players(&blackboard, PlayerNumber::One);

        assert_eq!(
            available,
            Players {
                one: false,
                two: false,
                three: true,
                four: false,
                five: false,
            },
        );
    }

    #[test]
    fn candidate_withdraws_in_stop() {
        let mut coordinator = ReplacementGoalkeeperCoordinator::default();
        let parameters = parameters();
        let input = playing_input(Time::zero());

        let stop_input = ReplacementGoalkeeperRoleInput {
            regular_goalkeeper_is_penalized: input.regular_goalkeeper_is_penalized,
            primary_state: PrimaryState::Stop,
            own_player_number: input.own_player_number,
            regular_goalkeeper_number: PlayerNumber::One,
            available_field_players: input.available_field_players,
        };

        coordinator.update(input, &parameters);

        assert_eq!(coordinator.state, ReplacementGoalkeeperState::Candidate,);
        assert!(coordinator.claims_role());
        assert!(coordinator.election_started_at.is_some());
        assert!(coordinator.replacement_needed_since.is_some());

        let selection_allowed = coordinator.refresh_role_status(stop_input);

        assert!(!selection_allowed);
        assert_eq!(coordinator.state, ReplacementGoalkeeperState::Inactive,);
        assert!(!coordinator.claims_role());
        assert_eq!(coordinator.known_replacement_goalkeeper, None);
        assert_eq!(coordinator.election_started_at, None);
        assert_eq!(coordinator.replacement_needed_since, None);
    }

    #[test]
    fn active_replacement_stays_active_in_stop() {
        let mut coordinator = ReplacementGoalkeeperCoordinator::default();
        let parameters = parameters();
        let input = playing_input(Time::zero());

        let stop_input = ReplacementGoalkeeperRoleInput {
            regular_goalkeeper_is_penalized: input.regular_goalkeeper_is_penalized,
            primary_state: PrimaryState::Stop,
            own_player_number: input.own_player_number,
            regular_goalkeeper_number: PlayerNumber::One,
            available_field_players: input.available_field_players,
        };

        // Wahl beginnen und nach 500 ms gewinnen.
        coordinator.update(input, &parameters);
        coordinator.update(
            playing_input(Time::from_nanos(500_000_000)), // duration of election window elapsed
            &parameters,
        );

        assert!(coordinator.is_active());
        assert_eq!(
            coordinator.known_replacement_goalkeeper,
            Some(PlayerNumber::Three),
        );

        let selection_allowed = coordinator.refresh_role_status(stop_input);

        assert!(!selection_allowed);
        assert!(coordinator.is_active());
        assert!(coordinator.claims_role());
        assert_eq!(
            coordinator.known_replacement_goalkeeper,
            Some(PlayerNumber::Three),
        );
        assert_eq!(coordinator.election_started_at, None);
        assert_eq!(coordinator.replacement_needed_since, None);
    }

    #[test]
    fn is_closest_available_field_player_to_own_goal_when_own_player_is_closest_to_goal() {
        let blackboard = goal_distance_blackboard();
        let available = calculate_available_field_players(&blackboard, PlayerNumber::One);

        // Eigene lokale Pose reicht; kein eigenes Netzwerk-Echo nötig.
        assert!(blackboard.world_state.player_states.three.is_none());
        assert!(is_closest_available_field_player_to_own_goal(
            &blackboard,
            &available
        ));
    }

    #[test]
    fn is_closest_available_field_player_to_own_goal_is_false_when_other_player_is_closer() {
        let mut blackboard = goal_distance_blackboard();
        blackboard
            .world_state
            .player_states
            .two
            .as_mut()
            .unwrap()
            .pose = Pose2::new(point!(-4.5, 0.0), 0.0);

        let available = calculate_available_field_players(&blackboard, PlayerNumber::One);

        assert!(!is_closest_available_field_player_to_own_goal(
            &blackboard,
            &available
        ));
    }

    #[test]
    fn is_closest_available_field_player_to_own_goal_ignores_unavailable_closer_player() {
        let mut blackboard = goal_distance_blackboard();
        blackboard
            .world_state
            .player_states
            .two
            .as_mut()
            .unwrap()
            .pose = Pose2::new(point!(-4.5, 0.0), 0.0);

        let mut available = calculate_available_field_players(&blackboard, PlayerNumber::One);
        available.two = false;

        assert!(is_closest_available_field_player_to_own_goal(
            &blackboard,
            &available
        ));
    }

    #[test]
    fn is_closest_available_field_player_to_own_goal_breaks_ties_by_player_number() {
        for (other_number, expected) in [(PlayerNumber::Two, false), (PlayerNumber::Four, true)] {
            let mut blackboard = goal_distance_blackboard();
            blackboard.world_state.player_states = Players::new(None);

            // Gleiche Position bedeutet exakt gleiche Entfernung.
            blackboard.world_state.player_states[other_number] = Some(PlayerState {
                pose: Pose2::new(point!(-4.0, 0.0), 0.0),
                head_yaw: 0.0,
                ball_position: None,
                replacement_goalkeeper_claim: false,
            });

            let available = calculate_available_field_players(&blackboard, PlayerNumber::One);

            assert_eq!(
                is_closest_available_field_player_to_own_goal(&blackboard, &available),
                expected,
                "tie with player {other_number:?}",
            );
        }
    }

    #[test]
    fn is_closest_available_field_player_to_own_goal_is_false_without_own_pose() {
        let mut blackboard = goal_distance_blackboard();
        blackboard.world_state.robot.ground_to_field = None;

        let available = calculate_available_field_players(&blackboard, PlayerNumber::One);

        assert!(!is_closest_available_field_player_to_own_goal(
            &blackboard,
            &available
        ));
    }

    #[test]
    fn finds_lowest_numbered_other_claiming_player() {
        let blackboard = blackboard_with_claims(&[PlayerNumber::Four, PlayerNumber::Two]);
        let available = calculate_available_field_players(&blackboard, PlayerNumber::One);

        assert_eq!(
            find_lowest_numbered_available_other_claiming_player(
                &blackboard,
                &available,
                PlayerNumber::One,
            ),
            Some(PlayerNumber::Two),
        );
    }

    #[test]
    fn ignores_own_claim_and_regular_goalkeeper_claim() {
        let blackboard =
            blackboard_with_claims(&[PlayerNumber::One, PlayerNumber::Three, PlayerNumber::Four]);

        // Bewusst alle verfügbar markieren, damit wir den Ausschluss
        // von Spieler 1 und uns selbst direkt in der Claim-Funktion testen.
        let available = Players::new(true);

        assert_eq!(
            find_lowest_numbered_available_other_claiming_player(
                &blackboard,
                &available,
                PlayerNumber::One
            ),
            Some(PlayerNumber::Four),
        );
    }

    #[test]
    fn ignores_claim_from_unavailable_player() {
        let blackboard = blackboard_with_claims(&[PlayerNumber::Two, PlayerNumber::Four]);
        let mut available = calculate_available_field_players(&blackboard, PlayerNumber::One);
        available.two = false;

        assert_eq!(
            find_lowest_numbered_available_other_claiming_player(
                &blackboard,
                &available,
                PlayerNumber::One
            ),
            Some(PlayerNumber::Four),
        );
    }

    #[test]
    fn returns_none_after_last_other_claim_is_withdrawn() {
        let mut blackboard = blackboard_with_claims(&[PlayerNumber::Two]);
        let available = calculate_available_field_players(&blackboard, PlayerNumber::One);

        assert_eq!(
            find_lowest_numbered_available_other_claiming_player(
                &blackboard,
                &available,
                PlayerNumber::One
            ),
            Some(PlayerNumber::Two),
        );

        blackboard
            .world_state
            .player_states
            .two
            .as_mut()
            .expect("test setup must contain player two")
            .replacement_goalkeeper_claim = false;

        assert_eq!(
            find_lowest_numbered_available_other_claiming_player(
                &blackboard,
                &available,
                PlayerNumber::One
            ),
            None,
        );
    }

    #[test]
    fn ignores_claim_after_player_state_is_removed() {
        let mut blackboard = blackboard_with_claims(&[PlayerNumber::Two, PlayerNumber::Four]);
        let available = calculate_available_field_players(&blackboard, PlayerNumber::One);

        assert_eq!(
            find_lowest_numbered_available_other_claiming_player(
                &blackboard,
                &available,
                PlayerNumber::One
            ),
            Some(PlayerNumber::Two),
        );

        // Simuliert das Entfernen eines abgelaufenen PlayerState.
        blackboard.world_state.player_states.two = None;

        assert_eq!(
            find_lowest_numbered_available_other_claiming_player(
                &blackboard,
                &available,
                PlayerNumber::One
            ),
            Some(PlayerNumber::Four),
        );
    }

    #[test]
    fn playing_tree_searches_without_election_when_keeper_is_available() {
        let mut blackboard = goal_distance_blackboard();
        blackboard.parameters.goalkeeper.player_number = PlayerNumber::One;
        blackboard.world_state.filtered_game_controller_state =
            Some(game_controller_state_for_tests());

        let tree = crate::tree::create_tree();
        let (status, _) = tree.tick_with_trace(&mut blackboard);

        assert_eq!(status, Status::Success);
        assert!(!blackboard.replacement_goalkeeper_coordinator.claims_role());
        assert_eq!(
            blackboard
                .replacement_goalkeeper_coordinator
                .replacement_needed_since,
            None,
        );

        // The normal search behavior must still run.
        assert!(matches!(
            blackboard.body_motion.as_ref(),
            Some(BodyMotion::WalkWithVelocity { .. }),
        ));
    }

    #[test]
    fn playing_tree_continues_searching_while_candidate() {
        let mut blackboard = goal_distance_blackboard();
        blackboard.parameters.goalkeeper.player_number = PlayerNumber::One;

        let mut game_controller_state = game_controller_state_for_tests();
        game_controller_state.penalties.one = Some(Penalty::PickUp {
            remaining: Duration::from_secs(30),
        });
        blackboard.world_state.filtered_game_controller_state = Some(game_controller_state);

        let tree = crate::tree::create_tree();
        let (status, _) = tree.tick_with_trace(&mut blackboard);

        assert_eq!(status, Status::Success);
        assert!(blackboard.replacement_goalkeeper_coordinator.claims_role());
        assert!(!blackboard.replacement_goalkeeper_coordinator.is_active());

        // Candidacy must not prevent the normal search behavior.
        assert!(matches!(
            blackboard.body_motion.as_ref(),
            Some(BodyMotion::WalkWithVelocity { .. }),
        ));
    }

    #[test]
    fn entering_simple_clears_candidate_and_active_replacement() {
        for initially_active in [false, true] {
            let mut blackboard = blackboard_with_candidate_or_active_replacement(initially_active);
            blackboard.parameters.control.is_simple = true;

            let tree = crate::tree::create_tree();
            let (status, _) = tree.tick_with_trace(&mut blackboard);

            assert_eq!(status, Status::Success);
            assert_eq!(
                blackboard.replacement_goalkeeper_coordinator,
                ReplacementGoalkeeperCoordinator::default(),
                "replacement must be reset; initially_active={initially_active}",
            );

            // Simple mode must continue searching when no ball is known.
            assert!(matches!(
                blackboard.body_motion.as_ref(),
                Some(BodyMotion::WalkWithVelocity { .. }),
            ));
        }
    }

    #[test]
    fn entering_penalty_shootout_clears_candidate_and_active_replacement() {
        for initially_active in [false, true] {
            let mut blackboard = blackboard_with_candidate_or_active_replacement(initially_active);

            blackboard
                .world_state
                .filtered_game_controller_state
                .as_mut()
                .expect("test setup must contain game controller state")
                .game_phase = GamePhase::PenaltyShootout {
                kicking_team: Team::Opponent,
            };

            let tree = crate::tree::create_tree();
            let (status, _) = tree.tick_with_trace(&mut blackboard);

            assert_eq!(status, Status::Success);
            assert_eq!(
                blackboard.replacement_goalkeeper_coordinator,
                ReplacementGoalkeeperCoordinator::default(),
                "replacement must be reset; initially_active={initially_active}",
            );

            // The existing shootout fallback must still run.
            assert!(matches!(
                blackboard.body_motion.as_ref(),
                Some(BodyMotion::Stand),
            ));
        }
    }

    #[test]
    fn candidate_withdraws_claim_when_tree_selects_striker() {
        let mut blackboard = blackboard_with_candidate_or_active_replacement(false);

        blackboard
            .parameters
            .network
            .remaining_amount_of_messages_to_stop_sending = 20;

        let game_controller_state = blackboard
            .world_state
            .filtered_game_controller_state
            .as_mut()
            .expect("test setup must contain game controller state");

        game_controller_state.remaining_number_of_messages = 1000;
        game_controller_state.half = hsl_network_messages::Half::Second;
        game_controller_state.remaining_time_in_half = Duration::from_secs(60);

        // Initially we are a candidate and advertise our claim.
        assert!(replacement_claim_from_next_state_message(&mut blackboard,));

        // The ball appears close to our position at x = -4.
        blackboard.world_state.now = Time::from_nanos(400_000_000);
        blackboard.ball = Some(LastBall {
            position: point!(-3.9, 0.0),
            velocity: Default::default(),
            age: blackboard.world_state.now,
            field_side: Side::Left,
            source: BallSource::Own,
        });

        blackboard.parameters.voronoi.grid_resolution = 0.25;

        // Test the role transition without waiting for entry hysteresis.
        blackboard.parameters.ball.closest_to_ball.enter_duration = Duration::ZERO;

        let tree = crate::tree::create_tree();
        let (status, _) = tree.tick_with_trace(&mut blackboard);

        assert_eq!(status, Status::Success);
        assert!(blackboard.last_closest_to_ball);
        assert!(!blackboard.replacement_goalkeeper_coordinator.claims_role());
        assert!(!blackboard.replacement_goalkeeper_coordinator.is_active());

        assert!(matches!(
            blackboard.body_motion.as_ref(),
            Some(BodyMotion::Kick { .. }),
        ));

        // The message cooldown has elapsed; advertise the withdrawn claim.
        assert!(!replacement_claim_from_next_state_message(&mut blackboard,));
    }

    #[test]
    fn two_coordinators_agree_on_lowest_claiming_player() {
        let mut coordinator_two = ReplacementGoalkeeperCoordinator::default();
        let mut coordinator_three = ReplacementGoalkeeperCoordinator::default();
        let parameters = parameters();

        let available = Players {
            one: false,
            two: true,
            three: true,
            four: false,
            five: false,
        };

        let input_for_player = |own_number, now, other_claim| ReplacementGoalkeeperInput {
            own_player_number: own_number,
            available_field_players: available,
            lowest_other_claiming_player: other_claim,
            ..playing_input(now)
        };

        // Both players start claiming before receiving the other's claim.
        coordinator_two.update(
            input_for_player(PlayerNumber::Two, Time::zero(), None),
            &parameters,
        );
        coordinator_three.update(
            input_for_player(PlayerNumber::Three, Time::zero(), None),
            &parameters,
        );

        assert!(coordinator_two.claims_role());
        assert!(coordinator_three.claims_role());
        assert!(!coordinator_two.is_active());
        assert!(!coordinator_three.is_active());

        for (now, election_finished) in [
            (Time::from_nanos(100_000_000), false),
            (Time::from_nanos(499_000_000), false),
            (Time::from_nanos(500_000_000), true),
        ] {
            // Capture both claims before updating either coordinator.
            let claim_from_two = coordinator_two.claims_role().then_some(PlayerNumber::Two);
            let claim_from_three = coordinator_three
                .claims_role()
                .then_some(PlayerNumber::Three);

            coordinator_two.update(
                input_for_player(PlayerNumber::Two, now, claim_from_three),
                &parameters,
            );
            coordinator_three.update(
                input_for_player(PlayerNumber::Three, now, claim_from_two),
                &parameters,
            );

            assert_eq!(coordinator_two.is_active(), election_finished);
            assert!(!coordinator_three.is_active());
        }

        assert!(coordinator_two.claims_role());
        assert!(!coordinator_three.claims_role());

        assert_eq!(
            coordinator_two.known_replacement_goalkeeper,
            Some(PlayerNumber::Two),
        );
        assert_eq!(
            coordinator_three.known_replacement_goalkeeper,
            Some(PlayerNumber::Two),
        );
    }

    #[test]
    fn fallback_starts_after_timeout_and_backoff_then_wins() {
        let mut coordinator = ReplacementGoalkeeperCoordinator::default();
        let parameters = parameters();

        let input_for_time = |now| ReplacementGoalkeeperInput {
            is_closest_available_field_player_to_own_goal: false,
            ..playing_input(now)
        };

        // Player three waits 2 seconds plus one backoff step:
        // player two is available and has a lower number.
        for now in [
            Time::zero(),
            Time::from_nanos(2_000_000_000),
            Time::from_nanos(2_199_000_000),
        ] {
            coordinator.update(input_for_time(now), &parameters);
            assert!(!coordinator.claims_role());
        }

        coordinator.update(input_for_time(Time::from_nanos(2_200_000_000)), &parameters);
        assert!(coordinator.claims_role());
        assert!(!coordinator.is_active());

        // The election window starts with the fallback claim.
        coordinator.update(input_for_time(Time::from_nanos(2_699_000_000)), &parameters);
        assert!(!coordinator.is_active());

        coordinator.update(input_for_time(Time::from_nanos(2_700_000_000)), &parameters);
        assert!(coordinator.is_active());
        assert_eq!(
            coordinator.known_replacement_goalkeeper,
            Some(PlayerNumber::Three),
        );
    }

    #[test]
    fn fallback_does_not_override_candidate_ineligibility() {
        let mut coordinator = ReplacementGoalkeeperCoordinator::default();
        let parameters = parameters();

        for now in [
            Time::zero(),
            Time::from_nanos(3_000_000_000),
            Time::from_nanos(10_000_000_000),
        ] {
            let input = ReplacementGoalkeeperInput {
                // Represents a player excluded by ball responsibility.
                is_eligible_candidate: false,
                is_closest_available_field_player_to_own_goal: true,
                ..playing_input(now)
            };

            coordinator.update(input, &parameters);

            assert!(!coordinator.claims_role());
            assert!(!coordinator.is_active());
            assert_eq!(coordinator.known_replacement_goalkeeper, None);
        }
    }
}
