use std::fs::read_dir;

use color_eyre::eyre::{Result, WrapErr};
use proc_macro2::TokenStream;
use quote::{format_ident, quote};

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
                    "control::center_of_mass_provider",
                    "control::filtered_game_controller_state_timer",
                    "control::game_controller_state_filter",
                    "control::ground_provider",
                    "control::kick_selector",
                    "control::odometry",
                    "control::kinematics_provider",
                    "control::motion::look_around",
                    "control::motion::motion_selector",
                    "control::motion::step_planner",
                    "control::motion::walk_manager",
                    "control::motion::walking_engine",
                    "control::primary_state_filter",
                    "control::referee_position_provider",
                    "control::role_assignment",
                    "control::rule_obstacle_composer",
                    "control::search_suggestor",
                    "control::support_foot_estimation",
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
    let paths: TokenStream = read_dir("../../tests/behavior")
        .expect("")
        .map(|file| {
            let path = file.unwrap().path();
            let name = path.file_stem().unwrap().to_string_lossy().clone();
            let test_file = path.file_name().unwrap().to_string_lossy().into_owned();
            let path = format!("../../tests/behavior/{test_file}");

            let function_name = format_ident!("test_{}", name);

            quote! {
                #[test]
                fn #function_name() -> color_eyre::Result<()> {
                    test_scenario(#path)
                }
            }
        })
        .collect();
    paths
        .write_to_file("behavior_files.rs")
        .wrap_err("failed to write generated tests to file")?;
    generate(&cyclers, &structs, ExecutionMode::Run)
        .write_to_file("generated_code.rs")
        .wrap_err("failed to write generated code to file")
}
