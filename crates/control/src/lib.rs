pub mod a_star;
pub mod active_vision;
pub mod ball_filter;
pub mod ball_state_composer;
pub mod behavior;
pub mod button_filter;
pub mod camera_matrix_calculator;
pub mod center_of_mass_provider;
pub mod fall_state_estimation;
pub mod game_controller_filter;
pub mod game_state_filter;
pub mod ground_contact_detector;
pub mod ground_provider;
pub mod kinematics_provider;
pub mod led_status;
pub mod limb_projector;
pub mod localization;
pub mod motion;
pub mod obstacle_filter;
pub mod odometry;
pub mod orientation_filter;
pub mod path_planner;
pub mod penalty_shot_direction_estimation;
pub mod primary_state_filter;
pub mod role_assignment;
pub mod sensor_data_receiver;
pub mod sole_pressure_filter;
pub mod sonar_filter;
pub mod spline_interpolator;
pub mod support_foot_estimation;
pub mod transition_interpolator;
pub mod whistle_filter;
pub mod world_state_composer;

#[derive(Clone, Copy, Debug)]
pub enum CyclerInstance {
    Control,
}
