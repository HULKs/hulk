use code_generation::{generate, write_to_file::WriteToFile, Execution};
use color_eyre::eyre::{Result, WrapErr};
use source_analyzer::{
    cyclers::{CyclerKind, Cyclers},
    manifest::{CyclerManifest, FrameworkManifest},
    pretty::to_string_pretty,
    structs::Structs,
};
use std::{fmt::Write, fs};

fn main() -> Result<()> {
    let manifest = FrameworkManifest {
        cyclers: vec![
            CyclerManifest {
                name: "Control",
                kind: CyclerKind::RealTime,
                instances: vec![""],
                setup_nodes: vec!["control::fake_data"],
                nodes: vec![
                    "control::active_vision",
                    "control::ball_state_composer",
                    "control::behavior::node",
                    "control::referee_position_provider",
                    "control::game_controller_state_filter",
                    "control::kick_selector",
                    "control::motion::look_around",
                    "control::referee_pose_detection_filter",
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
                nodes: vec![],
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
    let paths = fs::read_dir("../../tests/behavior")
        .expect("Failed to read directory")
        .fold(String::new(), |mut output, entry| {
            let test_file = entry
                .expect("Failed to read file name")
                .file_name()
                .to_string_lossy()
                .into_owned();
            let mut function_name = test_file.clone();
            function_name.truncate(function_name.len() - 4);
            let _ = write!(
                output,
                "#[test]\nfn test_{}() -> Result<()> {{\ntest_scenario(\"../../tests/behavior/{}\")}}\n",
                function_name, test_file
            );
            output
        });
    paths
        .write_to_file("behavior_files.rs")
        .wrap_err("failed to write generated tests to file")
        .and_then(|_| {
            generate(&cyclers, &structs, Execution::None)
                .write_to_file("generated_code.rs")
                .wrap_err("failed to write generated code to file")
        })
}
