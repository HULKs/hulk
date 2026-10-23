use std::{net::SocketAddr, time::Duration};

use hsl_network_messages::{
    GameControllerReturnMessage, GamePhase, Half, HulkMessage, PassIntent, StateMessage, SubState,
    Team,
};
use ros_z::time::Time;
use types::{
    fall_detection::Posture, filtered_game_controller_state::FilteredGameControllerState,
    messages::OutgoingMessage, parameters::HslNetworkParameters, primary_state::PrimaryState,
    time_wrapper::TimeWrapper,
};

use crate::{
    kick::{passing::matches_announced_pass, strategy::BallAction},
    node::Blackboard,
};

impl Blackboard {
    pub fn game_controller_return_message(
        &mut self,
        game_controller_address: Option<&SocketAddr>,
    ) -> Option<OutgoingMessage> {
        let now = self.world_state.now;

        if !self.is_return_message_cooldown_elapsed(now, &self.parameters.network) {
            return None;
        }
        let address = game_controller_address?;

        let ground_to_field = self.world_state.robot.ground_to_field.unwrap_or_default();

        let ball_position = self
            .world_state
            .ball
            .map(|ball| hsl_network_messages::BallPosition {
                age: now
                    .to_wallclock()
                    .duration_since(ball.last_seen_ball)
                    .unwrap(),
                position: ball.ball_in_ground,
            });

        self.last_sent_game_controller_return_message_time = Some(now);

        Some(OutgoingMessage::GameController(
            *address,
            GameControllerReturnMessage {
                player_number: self.player_number,
                fallen: self
                    .world_state
                    .fall_detection
                    .is_none_or(|fall_detection| {
                        !fall_detection.is_valid(now) || fall_detection.posture != Posture::Upright
                    }),
                pose: ground_to_field.as_pose(),
                ball: ball_position,
            },
        ))
    }

    fn is_return_message_cooldown_elapsed(
        &self,
        now: Time,
        hsl_network_parameters: &HslNetworkParameters,
    ) -> bool {
        is_cooldown_elapsed(
            now,
            self.last_sent_game_controller_return_message_time,
            hsl_network_parameters.game_controller_return_message_interval,
        )
    }

    pub fn try_sending_state_message(&mut self) -> Option<OutgoingMessage> {
        let preparing_kickoff = crate::kick::kickoff::needs_kickoff_updates(self);
        if self.world_state.robot.primary_state != PrimaryState::Playing
            && !(preparing_kickoff
                && matches!(
                    self.world_state.robot.primary_state,
                    PrimaryState::Ready | PrimaryState::Set
                ))
        {
            return None;
        }

        let now = self.world_state.now;
        let game_controller_state = self.world_state.filtered_game_controller_state.as_ref()?;
        let mut send_interval =
            state_message_send_interval(game_controller_state, &self.parameters.network)?;
        let replacement_goalkeeper_claim = self.replacement_goalkeeper_coordinator.claims_role();
        let keeper_claim_changed =
            replacement_goalkeeper_claim != self.last_sent_replacement_goalkeeper_claim;
        if self.replacement_goalkeeper_coordinator.is_electing() {
            send_interval = send_interval.min(Duration::from_millis(100));
        } else if self.replacement_goalkeeper_coordinator.keeper().is_some() {
            send_interval =
                send_interval.min(self.parameters.network.hsl_striker_message_receive_timeout / 2);
        }
        let preparing_restart = game_controller_state.kicking_team == Some(Team::Hulks)
            && game_controller_state
                .sub_state
                .is_some_and(|state| state != SubState::PenaltyKick);
        let preparing_pass = matches!(self.ball_action, Some(BallAction::Pass { .. }))
            && (preparing_kickoff
                || preparing_restart
                || self
                    .ball
                    .as_ref()
                    .zip(self.world_state.robot.ground_to_field)
                    .is_some_and(|(ball, transform)| {
                        (ball.position - transform.as_pose().position()).norm()
                            <= self.parameters.kicking.kick_activation_distance
                                + self.parameters.kicking.kick_activation_hysteresis
                    }));
        let outgoing_pass = crate::kick::passing::outgoing_pass_in_flight(self);
        let cancelling_pass =
            self.last_announced_pass.is_some() && !outgoing_pass && !preparing_pass;
        let announcing_pass = preparing_pass
            && self.world_state.robot.primary_state == PrimaryState::Playing
            && match (
                self.ball_action,
                self.kick_target,
                self.world_state.robot.ground_to_field,
            ) {
                (Some(BallAction::Pass { receiver }), Some(target), Some(transform)) => {
                    !matches_announced_pass(
                        self,
                        PassIntent {
                            receiver,
                            target: transform * target,
                        },
                    )
                }
                _ => false,
            };
        if matches!(self.ball_action, Some(BallAction::Pass { .. }))
            || preparing_kickoff
            || preparing_restart
            || outgoing_pass
            || cancelling_pass
        {
            // Keep pass preparation inside receiver freshness without spending the reserved budget.
            let preparation_interval = (self
                .parameters
                .kicking
                .strategy
                .passing
                .maximum_receiver_age
                / 2)
            .clamp(Duration::from_millis(100), PassIntent::TIMEOUT / 2);
            send_interval = send_interval.min(preparation_interval);
        }
        if cancelling_pass {
            send_interval = Duration::ZERO;
        }
        // Refresh the owner's pose once so teammates can recompute ball ownership promptly.
        if self.last_closest_to_ball
            && self
                .closest_to_ball_entered_area_since
                .is_some_and(|since| {
                    self.last_sent_hsl_message_time
                        .is_none_or(|last| last < since)
                })
        {
            send_interval =
                send_interval.min(self.parameters.network.silence_interval_between_messages);
        }
        if !announcing_pass
            && !keeper_claim_changed
            && !is_cooldown_elapsed(now, self.last_sent_hsl_message_time, send_interval)
        {
            return None;
        }
        if let Some(ground_to_field) = self.world_state.robot.ground_to_field {
            let pose = ground_to_field.as_pose();

            let ball_position =
                self.world_state
                    .ball
                    .map(|ball| hsl_network_messages::BallPosition {
                        age: now
                            .to_wallclock()
                            .duration_since(ball.last_seen_ball)
                            .unwrap(),
                        position: ball.ball_in_field,
                    });

            let pass = match (self.ball_action, self.kick_target) {
                (Some(BallAction::Pass { receiver }), Some(target))
                    if self.world_state.robot.primary_state == PrimaryState::Playing
                        && preparing_pass =>
                {
                    Some(PassIntent {
                        receiver,
                        target: ground_to_field * target,
                    })
                }
                _ => None,
            }
            .or_else(|| {
                self.last_announced_pass
                    .as_ref()
                    .filter(|_| outgoing_pass)
                    .map(|pass| pass.inner)
            });
            let announcement_time = pass
                .filter(|pass| matches_announced_pass(self, *pass))
                .and_then(|_| self.last_announced_pass.as_ref().map(|pass| pass.time))
                .unwrap_or(now);
            self.last_announced_pass = pass.map(|inner| TimeWrapper {
                time: announcement_time,
                inner,
            });
            let message = HulkMessage::State(StateMessage {
                player_number: self.player_number,
                pose,
                head_yaw: self.head_yaw,
                pose_age: self
                    .localization_time
                    .map(|localization_time| now.duration_since(localization_time)),
                ball_position,
                pass,
                replacement_goalkeeper_claim,
            });

            self.last_sent_hsl_message_time = Some(now);
            self.last_sent_replacement_goalkeeper_claim = replacement_goalkeeper_claim;
            Some(OutgoingMessage::Hsl(message))
        } else {
            None
        }
    }
    pub(crate) fn has_team_message_budget(&self) -> bool {
        self.world_state
            .filtered_game_controller_state
            .as_ref()
            .is_some_and(|state| {
                state.remaining_number_of_messages
                    > self
                        .parameters
                        .network
                        .remaining_amount_of_messages_to_stop_sending
            })
    }
}

pub(super) fn state_message_send_interval(
    game_controller_state: &FilteredGameControllerState,
    parameters: &HslNetworkParameters,
) -> Option<Duration> {
    let second_half_duration = match (game_controller_state.game_phase, game_controller_state.half)
    {
        (GamePhase::Normal, Half::First) => parameters.half_duration,
        (GamePhase::Extratime, Half::First) => parameters.extra_half_duration,
        _ => Duration::ZERO,
    };
    let remaining_time_in_game =
        game_controller_state.remaining_time_in_half + second_half_duration;
    let available_messages = game_controller_state
        .remaining_number_of_messages
        .saturating_sub(parameters.remaining_amount_of_messages_to_stop_sending);
    let active_player_count = game_controller_state
        .penalties
        .iter()
        .filter(|(_, penalty)| penalty.is_none())
        .count() as u32;

    if available_messages == 0 || active_player_count == 0 || remaining_time_in_game.is_zero() {
        return None;
    }

    Some(remaining_time_in_game * active_player_count / u32::from(available_messages))
}

fn is_cooldown_elapsed(now: Time, last: Option<Time>, cooldown: Duration) -> bool {
    match last {
        None => true,
        Some(last_time) => now.duration_since(last_time) > cooldown,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use types::filtered_game_state::FilteredGameState;

    #[test]
    fn new_ball_owner_refreshes_state_once_with_silence_and_budget_limits() {
        let mut board = Blackboard::default();
        board.world_state.robot.primary_state = PrimaryState::Playing;
        board.world_state.robot.ground_to_field = Some(Default::default());
        board.parameters.network.silence_interval_between_messages = Duration::from_secs(1);
        board
            .parameters
            .network
            .remaining_amount_of_messages_to_stop_sending = 20;
        board.world_state.filtered_game_controller_state = Some(FilteredGameControllerState {
            remaining_time_in_half: Duration::from_secs(600),
            remaining_number_of_messages: 100,
            ..crate::test_support::filtered_game_controller_state()
        });
        board.last_sent_hsl_message_time = Some(board.world_state.now);
        board.world_state.now = board.world_state.now + Duration::from_millis(500);
        board.closest_to_ball_entered_area_since = Some(board.world_state.now);
        board.last_closest_to_ball = true;
        assert!(board.try_sending_state_message().is_none());

        board.world_state.now = board.world_state.now + Duration::from_millis(600);
        let mut reserved = board.clone();
        reserved
            .world_state
            .filtered_game_controller_state
            .as_mut()
            .unwrap()
            .remaining_number_of_messages = 20;
        assert!(reserved.try_sending_state_message().is_none());
        let mut supporter = board.clone();
        supporter.last_closest_to_ball = false;
        assert!(supporter.try_sending_state_message().is_none());
        assert!(board.try_sending_state_message().is_some());
        board.world_state.now = board.world_state.now + Duration::from_millis(1100);
        assert!(board.try_sending_state_message().is_none());
    }

    #[test]
    fn restart_receivers_update_before_expiry_without_spending_reserved_messages() {
        let mut board = Blackboard::default();
        board.world_state.robot.primary_state = PrimaryState::Playing;
        board.world_state.robot.ground_to_field = Some(Default::default());
        board
            .parameters
            .kicking
            .strategy
            .passing
            .maximum_receiver_age = Duration::from_secs(3);
        board.parameters.network.half_duration = Duration::from_secs(600);
        board
            .parameters
            .network
            .remaining_amount_of_messages_to_stop_sending = 20;
        board.last_sent_hsl_message_time = Some(board.world_state.now);
        board.world_state.now = board.world_state.now + Duration::from_millis(1600);
        board.world_state.filtered_game_controller_state = Some(FilteredGameControllerState {
            game_state: FilteredGameState::Playing {
                ball_is_free: false,
                kick_off: false,
            },
            remaining_time_in_half: Duration::from_secs(600),
            remaining_number_of_messages: 100,
            kicking_team: Some(Team::Hulks),
            sub_state: Some(SubState::CornerKick),
            ..crate::test_support::filtered_game_controller_state()
        });
        assert!(
            state_message_send_interval(
                board
                    .world_state
                    .filtered_game_controller_state
                    .as_ref()
                    .unwrap(),
                &board.parameters.network,
            )
            .unwrap()
                > board
                    .parameters
                    .kicking
                    .strategy
                    .passing
                    .maximum_receiver_age
        );
        for state in [SubState::CornerKick, SubState::ThrowIn, SubState::GoalKick] {
            let mut receiver = board.clone();
            receiver
                .world_state
                .filtered_game_controller_state
                .as_mut()
                .unwrap()
                .sub_state = Some(state);
            assert!(receiver.try_sending_state_message().is_some(), "{state:?}");
            assert!(receiver.ball_action.is_none());
            assert!(receiver.last_announced_pass.is_none());
        }
        for case in 0..4 {
            let mut ordinary = board.clone();
            let game = ordinary
                .world_state
                .filtered_game_controller_state
                .as_mut()
                .unwrap();
            match case {
                0 => game.sub_state = None,
                1 => game.kicking_team = Some(Team::Opponent),
                2 => game.sub_state = Some(SubState::PenaltyKick),
                _ => game.remaining_number_of_messages = 20,
            }
            assert!(
                ordinary.try_sending_state_message().is_none(),
                "case {case}"
            );
        }
    }
}
