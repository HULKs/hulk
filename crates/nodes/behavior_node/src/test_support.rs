use std::time::Duration;

use hsl_network_messages::{Half, PlayerNumber};
use types::{
    field_dimensions::{FieldDimensions, GlobalFieldSide},
    filtered_game_controller_state::FilteredGameControllerState,
    filtered_game_state::FilteredGameState,
    parameters::{
        BehaviorParameters, GoalkeeperParameters, WalkSpeedParameters, WalkingBehaviorParameters,
    },
    world_state::PlayerState,
};

use crate::node::Blackboard;

impl Default for Blackboard {
    fn default() -> Self {
        Self {
            field_dimensions: field_dimensions(),
            player_number: PlayerNumber::Three,
            head_yaw: 0.0,
            parameters: behavior_parameters(),
            walking_velocity_limits: Default::default(),
            world_state: Default::default(),
            controller_input: None,
            remote_control_enabled: false,
            path_obstacles_output: Vec::new(),
            time_since_last_switch: Duration::ZERO,
            direction_difference: 0.0,
            voronoi_inputs: Vec::new(),
            ball: None,
            visual_kick_ball_position: None,
            last_ball: None,
            last_close_enough_to_kick: false,
            ball_recovery_side: None,
            ball_recovery_lane_cleared: false,
            kick_target: None,
            ball_action: None,
            last_kick_target: None,
            last_motion_command: None,
            last_motion_switch_time: Default::default(),
            last_motion_type: None,
            last_sent_game_controller_return_message_time: None,
            last_sent_hsl_message_time: None,
            last_announced_pass: None,
            replacement_goalkeeper_coordinator: Default::default(),
            last_sent_replacement_goalkeeper_claim: false,
            kickoff_receiver_wait_started: None,
            restart_pass_wait_started: None,
            kickoff_prepared: false,
            kickoff_started: None,
            localization_time: None,
            search_cell_size: None,
            last_closest_to_ball: false,
            closest_to_ball_entered_area_since: None,
            closest_to_ball_left_area_since: None,
            is_injected_motion_command: false,
            walk_position: None,
            body_motion: None,
            head_motion: None,
            voronoi_map: None,
        }
    }
}

pub(crate) fn behavior_parameters() -> BehaviorParameters {
    BehaviorParameters {
        control: Default::default(),
        search: Default::default(),
        ball: Default::default(),
        walking: WalkingBehaviorParameters {
            path_planning: Default::default(),
            walk_and_stand: Default::default(),
            speed: WalkSpeedParameters {
                kicking: 1.0,
                search: 1.0,
                blocking: 1.0,
                minimum_speed: 0.2,
                velocity_fade_distance: 1.0,
                walk_to_kickoff: 0.5,
            },
        },
        kicking: Default::default(),
        goalkeeper: goalkeeper_parameters(),
        substates: Default::default(),
        kickoff: Default::default(),
        voronoi: Default::default(),
        network: Default::default(),
    }
}

pub(crate) fn goalkeeper_parameters() -> GoalkeeperParameters {
    GoalkeeperParameters {
        player_number: PlayerNumber::Three,
        x_offset: 0.0,
        passive_distance: 0.0,
        striker_distance: 0.0,
        kick_away_ball_maximum_robot_distance: 0.0,
        active_defense_maximum_robot_distance: 0.0,
        distance_to_goalpost: 0.0,
    }
}

pub(crate) fn filtered_game_controller_state() -> FilteredGameControllerState {
    FilteredGameControllerState {
        game_state: FilteredGameState::Initial,
        opponent_game_state: FilteredGameState::Initial,
        half: Half::First,
        remaining_time_in_half: Duration::ZERO,
        game_phase: Default::default(),
        kicking_team: None,
        penalties: Default::default(),
        remaining_number_of_messages: 0,
        sub_state: None,
        global_field_side: GlobalFieldSide::Away,
        new_own_penalties_last_cycle: Default::default(),
        new_opponent_penalties_last_cycle: Default::default(),
    }
}

pub(crate) fn player_state() -> PlayerState {
    PlayerState {
        pose: Default::default(),
        head_yaw: 0.0,
        pose_age: None,
        ball_position: None,
        pass: None,
        replacement_goalkeeper_claim: false,
    }
}

pub(crate) fn field_dimensions() -> FieldDimensions {
    FieldDimensions {
        ball_radius: 0.0,
        length: 0.0,
        width: 0.0,
        line_width: 0.0,
        penalty_marker_size: 0.0,
        goal_box_area_length: 0.0,
        goal_box_area_width: 0.0,
        penalty_area_length: 0.0,
        penalty_area_width: 0.0,
        penalty_marker_distance: 0.0,
        center_circle_diameter: 0.0,
        border_strip_width: 0.0,
        goal_inner_width: 0.0,
        goal_post_diameter: 0.0,
        goal_depth: 0.0,
        corner_arc_radius: 0.0,
    }
}
