use code_generation::{generate, write_to_file::WriteToFile, Execution};
use color_eyre::eyre::{Result, WrapErr};
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
                setup_nodes: vec!["control::fake_data"],
                nodes: vec![
                    "control::active_vision",
                    "control::ball_state_composer",
                    "control::behavior::node",
                    "control::game_controller_state_filter",
                    "control::kick_selector",
                    "control::motion::look_around",
                    "control::role_assignment",
                    "control::rule_obstacle_composer",
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
    generate(&cyclers, &structs, Execution::None)
        .write_to_file("generated_code.rs")
        .wrap_err("failed to write generated code to file")
}
