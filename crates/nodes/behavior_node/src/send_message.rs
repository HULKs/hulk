use std::{collections::HashMap, net::SocketAddr, time::Duration};

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
    kick::{
        passing::{PASS_EXECUTION_TIMEOUT, is_executing_announced_pass, matches_announced_pass},
        strategy::BallAction,
    },
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
        let preparing_restart = game_controller_state.kicking_team == Some(Team::Hulks)
            && game_controller_state
                .sub_state
                .is_some_and(|state| state != SubState::PenaltyKick);
        let preparing_pass = matches!(self.ball_action, Some(BallAction::Pass { .. }))
            && (preparing_kickoff
                || preparing_restart
                || match self.ball_action {
                    Some(BallAction::Pass { receiver }) => {
                        is_executing_announced_pass(self, receiver)
                    }
                    _ => false,
                });
        let outgoing_pass = crate::kick::passing::outgoing_pass_in_flight(self);
        let receiving_pass = crate::kick::passing::incoming_pass(self).is_some();
        let pass = match (
            self.ball_action,
            self.kick_target,
            self.world_state.robot.ground_to_field,
        ) {
            (Some(BallAction::Pass { receiver }), Some(target), Some(transform))
                if self.world_state.robot.primary_state == PrimaryState::Playing
                    && preparing_pass =>
            {
                Some(PassIntent {
                    receiver,
                    target: transform * target,
                })
            }
            _ => None,
        }
        .or_else(|| {
            self.last_announced_pass
                .as_ref()
                .filter(|_| outgoing_pass)
                .map(|pass| pass.inner)
        })
        .filter(|pass| pass.target.inner.iter().all(|value| value.is_finite()))
        .filter(|pass| {
            self.last_announced_pass.as_ref().is_none_or(|announced| {
                announced.inner.receiver != pass.receiver
                    || (announced.inner.target - pass.target).norm()
                        > self.parameters.kicking.strategy.passing.target_half_width
                    || (announced.time <= now
                        && now.duration_since(announced.time) < PASS_EXECUTION_TIMEOUT)
            })
        });
        let cancelling_pass = self.last_announced_pass.is_some() && pass.is_none();
        let announcing_pass = pass.is_some_and(|pass| !matches_announced_pass(self, pass));
        if matches!(self.ball_action, Some(BallAction::Pass { .. }))
            || preparing_kickoff
            || preparing_restart
            || outgoing_pass
            || receiving_pass
            || cancelling_pass
        {
            // Refresh before the robot receiver's one-second retention expires.
            let preparation_interval = (self
                .parameters
                .kicking
                .strategy
                .passing
                .maximum_receiver_age
                / 2)
            .clamp(Duration::from_millis(100), Duration::from_millis(500));
            send_interval = send_interval.min(preparation_interval);
        }
        if cancelling_pass {
            send_interval = Duration::ZERO;
        }
        if !announcing_pass
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

            // Rebroadcasts refresh peer freshness, never the local wind-up/flight bound.
            let announcement_time = pass
                .and_then(|pass| {
                    self.last_announced_pass
                        .as_ref()
                        .filter(|announced| {
                            announced.inner.receiver == pass.receiver
                                && (announced.inner.target - pass.target).norm()
                                    <= self.parameters.kicking.strategy.passing.target_half_width
                        })
                        .map(|announced| announced.time)
                })
                .unwrap_or(now);
            self.last_announced_pass = pass.map(|inner| TimeWrapper {
                time: announcement_time,
                inner,
            });
            let message = HulkMessage::State(StateMessage {
                player_number: self.player_number,
                pose,
                head_yaw: self.head_yaw,
                ball_position,
                pass,
            });

            self.last_sent_hsl_message_time = Some(now);
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

pub(crate) fn recent_game_controller_address(
    contacts: &HashMap<SocketAddr, Time>,
    now: Time,
    timeout: Duration,
) -> Option<SocketAddr> {
    contacts
        .iter()
        .max_by_key(|(_, time)| *time)
        .filter(|(_, time)| now.duration_since(**time) < timeout)
        .map(|(address, _)| *address)
}

pub(crate) fn state_message_send_interval(
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
        let receiver_parameters: HashMap<String, Duration> = json5::from_str(include_str!(
            "../../../../etc/parameters/base/player_states_receiver.json5"
        ))
        .unwrap();
        let update_age = receiver_parameters["maximum_age"].mul_f32(0.6);
        board.world_state.now = board.world_state.now + update_age;
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
    #[test]
    fn addressed_receivers_keep_refreshing_after_restart_release() {
        let mut board = crate::test_support::restart_blackboard(
            SubState::DirectFreeKick,
            linear_algebra::Point2::origin(),
        );
        let game = board
            .world_state
            .filtered_game_controller_state
            .as_mut()
            .unwrap();
        game.sub_state = None;
        game.remaining_time_in_half = Duration::from_secs(600);
        game.remaining_number_of_messages = 100;
        assert!(
            state_message_send_interval(game, &board.parameters.network).unwrap()
                > board
                    .parameters
                    .kicking
                    .strategy
                    .passing
                    .maximum_receiver_age
        );
        let own_position = board
            .world_state
            .robot
            .ground_to_field
            .unwrap()
            .as_pose()
            .position();
        board.world_state.player_states.two = Some(TimeWrapper {
            time: board.world_state.now,
            inner: types::world_state::PlayerState {
                pose: linear_algebra::point![-3.0, 0.0].into(),
                pass: Some(PassIntent {
                    receiver: board.player_number,
                    target: own_position,
                }),
                ..crate::test_support::player_state()
            },
        });
        board.last_sent_hsl_message_time = Some(board.world_state.now);
        board.world_state.now = board.world_state.now + Duration::from_millis(600);
        assert!(crate::kick::passing::incoming_pass(&board).is_some());
        assert!(board.ball_action.is_none());
        assert!(board.try_sending_state_message().is_some());

        board.world_state.now = board.world_state.now + Duration::from_millis(600);
        board
            .world_state
            .filtered_game_controller_state
            .as_mut()
            .unwrap()
            .remaining_number_of_messages = board
            .parameters
            .network
            .remaining_amount_of_messages_to_stop_sending;
        assert!(board.try_sending_state_message().is_none());
    }

    #[test]
    fn rebroadcasts_keep_original_announcement_even_during_a_restart_and_reselection_is_immediate()
    {
        let mut board = crate::test_support::restart_blackboard(
            SubState::DirectFreeKick,
            linear_algebra::Point2::origin(),
        );
        let transform = board.world_state.robot.ground_to_field.unwrap();
        board.ball_action = Some(BallAction::Pass {
            receiver: hsl_network_messages::PlayerNumber::Two,
        });
        board.kick_target = Some(transform.inverse() * linear_algebra::point![2.0, 0.0]);
        board.last_sent_hsl_message_time = Some(board.world_state.now);
        assert!(board.try_sending_state_message().is_some());
        let announced_at = board.last_announced_pass.as_ref().unwrap().time;
        let mut changed = board.clone();
        changed.world_state.now = changed.world_state.now + Duration::from_millis(1);
        changed.kick_target = Some(transform.inverse() * linear_algebra::point![2.0, 1.0]);
        assert!(changed.try_sending_state_message().is_some());
        assert_eq!(
            changed.last_announced_pass.as_ref().unwrap().time,
            changed.world_state.now
        );
        for age in [1600, 3200, 4800] {
            board.world_state.now = announced_at + Duration::from_millis(age);
            assert!(board.try_sending_state_message().is_some());
            assert_eq!(
                board.last_announced_pass.as_ref().unwrap().time,
                announced_at
            );
        }
        board.world_state.now = announced_at + PASS_EXECUTION_TIMEOUT;
        let Some(OutgoingMessage::Hsl(HulkMessage::State(message))) =
            board.try_sending_state_message()
        else {
            panic!("expired pass must be cancelled");
        };
        assert!(message.pass.is_none());
        assert!(board.last_announced_pass.is_none());
    }
}
