use color_eyre::eyre::{Result, WrapErr};

use code_generation::{generate, write_to_file::WriteToFile, ExecutionMode};
use source_analyzer::{
    cyclers::{CyclerKind, Cyclers},
    manifest::{CyclerManifest, FrameworkManifest},
    pretty::to_string_pretty,
    structs::Structs,
};

fn main() -> Result<()> {
    let manifest = FrameworkManifest {
        cyclers: vec![
            CyclerManifest {
                name: "Control",
                kind: CyclerKind::RealTime,
                instances: vec![""],
                setup_nodes: vec!["crate::fake_data"],
                nodes: vec![
                    "control::active_vision",
                    "control::ball_state_composer",
                    "control::behavior::node",
                    "control::game_controller_state_filter",
                    "control::kick_selector",
                    "control::kicking_team_filter",
                    "control::free_kick_signal_filter",
                    "control::filtered_game_controller_state_timer",
                    "control::primary_state_filter",
                    "control::motion::look_around",
                    "control::motion::motion_selector",
                    "control::penalty_shot_direction_estimation",
                    "control::referee_position_provider",
                    "control::role_assignment",
                    "control::rule_obstacle_composer",
                    "control::search_suggestor",
                    "control::time_to_reach_kick_position",
                    "control::world_state_composer",
                ],
            },
            CyclerManifest {
                name: "SplNetwork",
                kind: CyclerKind::Perception,
                instances: vec![""],
                setup_nodes: vec!["spl_network::message_receiver"],
                nodes: vec!["spl_network::message_filter"],
            },
            CyclerManifest {
                name: "ObjectDetection",
                kind: CyclerKind::Perception,
                instances: vec!["Top"],
                setup_nodes: vec!["vision::image_receiver"],
                nodes: vec![
                    "object_detection::pose_detection",
                    "object_detection::pose_filter",
                    "object_detection::pose_interpretation",
                ],
            },
        ],
    };
    let root = "../../crates/";

    let cyclers = Cyclers::try_from_manifest(manifest, root)?;
    for path in cyclers.watch_paths() {
        println!("cargo:rerun-if-changed={}", path.display());
    }

    println!();
    println!("{}", to_string_pretty(&cyclers)?);

    let structs = Structs::try_from_cyclers(&cyclers)?;
    generate(&cyclers, &structs, ExecutionMode::Run)
        .write_to_file("generated_code.rs")
        .wrap_err("failed to write generated code to file")
}
