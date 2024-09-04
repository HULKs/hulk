use color_eyre::eyre::{Result, WrapErr};

use code_generation::{generate, write_to_file::WriteToFile, ExecutionMode};
use hulk_manifest::collect_hulk_cyclers;
use source_analyzer::{pretty::to_string_pretty, structs::Structs};

fn main() -> Result<()> {
    let cyclers = collect_hulk_cyclers()?;
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
