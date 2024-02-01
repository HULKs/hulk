use code_generation::{generate, write_to_file::WriteToFile, Execution};
use color_eyre::eyre::{Result, WrapErr};
use hulk_manifest::collect_hulk_cyclers;
use source_analyzer::{pretty::to_string_pretty, structs::Structs};

fn main() -> Result<()> {
    #[allow(unused_mut)] // must not be mut if "with_detection" feature is disabled
    let mut cyclers = collect_hulk_cyclers()?;
    #[cfg(not(feature = "with_object_detection"))]
    cyclers
        .cyclers
        .retain(|cycler| cycler.name != "ObjectDetection");
    for path in cyclers.watch_paths() {
        println!("cargo:rerun-if-changed={}", path.display());
    }

    println!();
    println!("{}", to_string_pretty(&cyclers)?);

    let structs = Structs::try_from_cyclers(&cyclers)?;
    generate(&cyclers, &structs, Execution::Replay)
        .write_to_file("generated_code.rs")
        .wrap_err("failed to write generated code to file")
}
