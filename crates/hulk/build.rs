use code_generation::{collect_watch_paths, generate, write_to_file::WriteToFile};
use color_eyre::eyre::{Result, WrapErr};
use source_analyzer::{
    cyclers::{CyclerKind, Cyclers},
    manifest::{CyclerManifest, FrameworkManifest},
    structs::Structs,
};

fn main() -> Result<()> {
    println!("cargo:rerun-if-changed=framework.toml");
    let manifest = FrameworkManifest::default()
        .cycler(
            CyclerManifest::new("Vision", CyclerKind::Perception)
                .instance("Top".to_string())
                .instance("Bottom".to_string())
                .setup_node("vision::image_receiver")?
                .node("vision::ball_detection")?
                .node("vision::camera_matrix_extractor")?
                .node("vision::feet_detection")?
                .node("vision::field_border_detection")?
                .node("vision::field_color_detection")?
                .node("vision::image_segmenter")?
                .node("vision::limb_projector")?
                .node("vision::line_detection")?
                .node("vision::perspective_grid_candidates_provider")?
                .node("vision::robot_detection")?
                .node("vision::segment_filter")?,
        )
        .cycler(
            CyclerManifest::new("Control", CyclerKind::RealTime)
                .instance("".to_string())
                .setup_node("control::sensor_data_receiver")?
                .node("control::active_vision")?
                .node("control::ball_filter")?
                .node("control::ball_state_composer")?
                .node("control::behavior::node")?
                .node("control::button_filter")?
                .node("control::camera_matrix_calculator")?
                .node("control::center_of_mass_provider")?
                .node("control::fall_state_estimation")?
                .node("control::game_controller_filter")?
                .node("control::game_state_filter")?
                .node("control::ground_contact_detector")?
                .node("control::ground_provider")?
                .node("control::hulk_message_filter")?
                .node("control::kick_selector")?
                .node("control::kinematics_provider")?
                .node("control::led_status")?
                .node("control::localization")?
                .node("control::motion::arms_up_squat")?
                .node("control::motion::condition_input_provider")?
                .node("control::motion::dispatching_interpolator")?
                .node("control::motion::energy_saving_stand")?
                .node("control::motion::fall_protector")?
                .node("control::motion::head_motion")?
                .node("control::motion::joint_command_sender")?
                .node("control::motion::jump_left")?
                .node("control::motion::jump_right")?
                .node("control::motion::look_around")?
                .node("control::motion::look_at")?
                .node("control::motion::motion_selector")?
                .node("control::motion::sit_down")?
                .node("control::motion::stand_up_back")?
                .node("control::motion::stand_up_front")?
                .node("control::motion::step_planner")?
                .node("control::motion::walk_manager")?
                .node("control::motion::walking_engine")?
                .node("control::obstacle_filter")?
                .node("control::odometry")?
                .node("control::orientation_filter")?
                .node("control::penalty_shot_direction_estimation")?
                .node("control::primary_state_filter")?
                .node("control::role_assignment")?
                .node("control::rule_obstacle_composer")?
                .node("control::sole_pressure_filter")?
                .node("control::sonar_filter")?
                .node("control::support_foot_estimation")?
                .node("control::whistle_filter")?
                .node("control::world_state_composer")?,
        )
        .cycler(
            CyclerManifest::new("SplNetwork", CyclerKind::Perception)
                .instance("".to_string())
                .setup_node("spl_network::message_receiver")?,
        )
        .cycler(
            CyclerManifest::new("Audio", CyclerKind::Perception)
                .instance("".to_string())
                .setup_node("audio::microphone_recorder")?
                .node("audio::whistle_detection")?,
        );
    let root = "..";

    for path in collect_watch_paths(&manifest) {
        println!("cargo:rerun-if-changed={root}/{}", path.display());
    }

    let mut cyclers = Cyclers::try_from_manifest(manifest, root)?;
    cyclers.sort_nodes()?;
    let structs = Structs::try_from_cyclers(&cyclers)?;
    generate(&cyclers, &structs)
        .write_to_file("generated_code.rs")
        .wrap_err("failed to write generated code to file")
}
