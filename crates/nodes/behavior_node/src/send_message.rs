use std::{f32::consts::PI, net::SocketAddr, time::Duration};

use booster::FallDownStateType;
use hsl_network_messages::{GameControllerReturnMessage, Half, HulkMessage, StateMessage};
use ros_z::time::Time;
use types::{
    messages::OutgoingMessage, parameters::HslNetworkParameters, primary_state::PrimaryState,
};

use crate::node::Blackboard;

impl Blackboard {
    pub fn game_controller_return_message(
        &mut self,
        game_controller_address: Option<&SocketAddr>,
    ) -> Option<OutgoingMessage> {
        let now = self.world_state.now;

        if !self.is_return_message_cooldown_elapsed(now, &self.parameters.hsl_network) {
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
                player_number: self.world_state.robot.player_number,
                fallen: self
                    .world_state
                    .fall_down_state
                    .is_some_and(|state| state.fall_down_state != FallDownStateType::IsReady),
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

    pub fn state_message(&mut self) -> Option<OutgoingMessage> {
        let now = self.world_state.now;
        let filtered_game_controller_state =
            self.world_state.filtered_game_controller_state.as_ref()?;

        if !self.is_state_message_cooldown_elapsed(now, &self.parameters.hsl_network) {
            return None;
        }
        if filtered_game_controller_state.remaining_number_of_messages
            < self
                .parameters
                .hsl_network
                .remaining_amount_of_messages_to_stop_sending
        {
            return None;
        }
        if self.world_state.robot.primary_state != PrimaryState::Playing {
            return None;
        }

        let ground_to_field = self.world_state.robot.ground_to_field?;
        let pose = ground_to_field.as_pose();

        let ball_position = self
            .world_state
            .ball
            .map(|ball| hsl_network_messages::BallPosition {
                age: now
                    .to_wallclock()
                    .duration_since(ball.last_seen_ball)
                    .unwrap(),
                position: ball.ball_in_field,
            });

        let message = HulkMessage::State(StateMessage {
            player_number: self.world_state.robot.player_number,
            pose,
            ball_position,
        });

        let max_difference_scale = message_difference_scale(
            filtered_game_controller_state.half,
            filtered_game_controller_state.remaining_time_in_half,
            filtered_game_controller_state.remaining_number_of_messages,
            &self.parameters.hsl_network,
        )?;

        if !is_message_different(
            &message,
            self.last_sent_hsl_message.as_ref(),
            max_difference_scale,
        ) && self
            .last_sent_hsl_message_time
            .is_some_and(|last_sent_hsl_message_time| {
                now.duration_since(last_sent_hsl_message_time)
                    < self.parameters.hsl_network.max_time_since_last_message
            })
        {
            return None;
        }

        self.last_sent_hsl_message = Some(message);
        self.last_sent_hsl_message_time = Some(now);

        Some(OutgoingMessage::Hsl(message))
    }

    fn is_state_message_cooldown_elapsed(
        &self,
        now: Time,
        hsl_network_parameters: &HslNetworkParameters,
    ) -> bool {
        is_cooldown_elapsed(
            now,
            self.last_sent_hsl_message_time,
            hsl_network_parameters.hsl_state_message_send_interval,
        )
    }
}

fn is_cooldown_elapsed(now: Time, last: Option<Time>, cooldown: Duration) -> bool {
    match last {
        None => true,
        Some(last_time) => now.duration_since(last_time) > cooldown,
    }
}

fn message_difference_scale(
    half: Half,
    mut remaining_time: Duration,
    remaining_number_of_messages: u16,
    hsl_network_parameters: &HslNetworkParameters,
) -> Option<f32> {
    let remaining_messages = remaining_number_of_messages as isize
        - hsl_network_parameters.remaining_amount_of_messages_to_stop_sending as isize;

    if half == Half::First {
        remaining_time += hsl_network_parameters.half_duration;
    }
    if remaining_messages <= 0 {
        return None;
    }

    let full_game_duration = hsl_network_parameters.half_duration.mul_f32(2.0);
    if full_game_duration.is_zero() {
        return None;
    }

    remaining_time = remaining_time.min(full_game_duration);
    let expected_remaining_messages = hsl_network_parameters.message_budget as f32
        * remaining_time.as_secs_f32()
        / full_game_duration.as_secs_f32();
    let remaining_message_ratio = expected_remaining_messages / remaining_messages as f32;

    Some(hsl_network_parameters.max_message_difference_scale * remaining_message_ratio)
}

fn is_message_different(
    message: &HulkMessage,
    last_sent_message: Option<&HulkMessage>,
    max_difference_scale: f32,
) -> bool {
    let Some(last_sent_message) = last_sent_message else {
        return true;
    };

    let (HulkMessage::State(message), HulkMessage::State(last_message)) =
        (message, last_sent_message);
    let pose_position_difference = (message.pose.position() - last_message.pose.position()).norm();
    let pose_angle_difference = angular_difference(
        message.pose.orientation().angle(),
        last_message.pose.orientation().angle(),
    );

    let ball_position_difference = match (message.ball_position, last_message.ball_position) {
        (None, None) => 0.0,
        (Some(left), Some(right)) => (left.position - right.position).norm(),
        _ => f32::INFINITY,
    };

    pose_position_difference > max_difference_scale
        || pose_angle_difference > max_difference_scale
        || ball_position_difference > max_difference_scale
}

fn angular_difference(from: f32, to: f32) -> f32 {
    ((from - to + PI).rem_euclid(2.0 * PI) - PI).abs()
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use coordinate_systems::Ground;
    use hsl_network_messages::PlayerNumber;
    use linear_algebra::{Isometry2, Point2, point, vector};
    use types::{
        field_dimensions::FieldDimensions,
        filtered_game_controller_state::FilteredGameControllerState,
        motion_command::MotionCommand,
        motion_type::MotionType,
        parameters::BehaviorParameters,
        world_state::{BallState, RobotState, WorldState},
    };

    use super::*;

    #[test]
    fn message_difference_scale_uses_full_game_budget_units() {
        let hsl_network_parameters = hsl_network_parameters();

        let scale = message_difference_scale(
            Half::Second,
            Duration::from_secs(600),
            600,
            &hsl_network_parameters,
        )
        .unwrap();

        assert!((scale - 0.3).abs() < f32::EPSILON);
    }

    #[test]
    fn state_message_is_sent_for_change_or_heartbeat() {
        let mut blackboard = blackboard_for_state_message();
        allow_recheck_before_timeout(&mut blackboard);

        assert!(blackboard.state_message().is_some());
        blackboard.world_state.now = Time::from_nanos(1_100_000_000);
        assert!(blackboard.state_message().is_none());

        blackboard.world_state.robot.ground_to_field =
            Some(Isometry2::from_parts(vector![10.0, 0.0], 0.0));
        assert!(blackboard.state_message().is_some());

        let mut blackboard = blackboard_for_state_message();
        allow_recheck_before_timeout(&mut blackboard);
        blackboard.world_state.ball = Some(ball_state(0.0, 0.0));

        assert!(blackboard.state_message().is_some());
        blackboard.world_state.now = Time::from_nanos(1_100_000_000);
        blackboard.world_state.ball = Some(ball_state(10.0, 0.0));

        assert!(blackboard.state_message().is_some());

        let mut blackboard = blackboard_for_state_message();
        allow_recheck_before_timeout(&mut blackboard);

        assert!(blackboard.state_message().is_some());
        blackboard.world_state.now = Time::from_nanos(3_000_000_000);

        assert!(blackboard.state_message().is_some());
    }

    fn allow_recheck_before_timeout(blackboard: &mut Blackboard) {
        blackboard
            .parameters
            .hsl_network
            .hsl_state_message_send_interval = Duration::from_millis(1);
    }

    fn hsl_network_parameters() -> HslNetworkParameters {
        HslNetworkParameters {
            message_budget: 1_200,
            half_duration: Duration::from_secs(600),
            max_time_since_last_message: Duration::from_secs(2),
            max_message_difference_scale: 0.3,
            ..Default::default()
        }
    }

    fn ball_state(x: f32, y: f32) -> BallState {
        BallState {
            ball_in_ground: point![x, y],
            ball_in_field: point![x, y],
            ..Default::default()
        }
    }

    fn blackboard_for_state_message() -> Blackboard {
        let mut world_state = WorldState {
            now: Time::from_nanos(1_000_000_000),
            filtered_game_controller_state: Some(FilteredGameControllerState {
                half: Half::Second,
                remaining_time_in_half: Duration::from_secs(600),
                remaining_number_of_messages: 1_200,
                ..Default::default()
            }),
            robot: RobotState {
                ground_to_field: Some(Isometry2::identity()),
                player_number: PlayerNumber::Three,
                primary_state: PrimaryState::Playing,
            },
            ..Default::default()
        };
        world_state.ball = None;

        let mut parameters = BehaviorParameters::default();
        parameters.hsl_network = hsl_network_parameters();

        Blackboard {
            field_dimensions: FieldDimensions::SPL_2025,
            parameters,
            world_state,
            path_obstacles_output: Vec::new(),
            time_since_last_switch: Duration::ZERO,
            direction_difference: 0.0,
            voronoi_inputs: Vec::new(),
            ball: None,
            last_ball: None,
            last_close_enough_to_kick: false,
            last_kick_target: None,
            last_motion_command: MotionCommand::default(),
            last_motion_switch_time: Time::zero(),
            last_motion_type: None::<MotionType>,
            last_sent_game_controller_return_message_time: None,
            last_sent_hsl_message: None,
            last_sent_hsl_message_time: None,
            last_closest_to_ball: false,
            closest_to_ball_entered_area_since: None,
            closest_to_ball_left_area_since: None,
            is_injected_motion_command: false,
            walk_position: None::<Point2<Ground>>,
            body_motion: None,
            head_motion: None,
            voronoi_map: None,
        }
    }
}
