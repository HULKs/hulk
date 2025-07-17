use color_eyre::eyre::{Result, WrapErr};

use code_generation::{structs::generate_structs, write_to_file::WriteToFile};
use hulk_manifest::collect_hulk_cyclers;
use source_analyzer::{pretty::to_string_pretty, structs::Structs};

fn main() -> Result<()> {
    let cyclers = collect_hulk_cyclers("../../crates/")?;
    for path in cyclers.watch_paths() {
        println!("cargo:rerun-if-changed={}", path.display());
    }

    println!();
    println!("{}", to_string_pretty(&cyclers)?);

    let structs = Structs::try_from_cyclers(&cyclers)?;
    generate_structs(&structs)
        .write_to_file("generated_code.rs")
        .wrap_err("failed to write generated code to file")
}
