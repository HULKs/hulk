use std::env::var;

use code_generation::{framework::generate_framework, write_to_file::WriteToFile};
use color_eyre::eyre::{Result, WrapErr};
use source_analyzer::{cycler::Cyclers, structs::Structs};

fn main() -> Result<()> {
    let framework_manifest =
        var("FRAMEWORK_MANIFEST_PATH").unwrap_or_else(|_| "framework.toml".to_string());
    let cyclers = Cyclers::try_from_toml(framework_manifest)?;
    let structs = Structs::try_from_cyclers(&cyclers)?;
    generate_framework(&cyclers, &structs)
        .write_to_file("generated_framework.rs")
        .wrap_err("failed to write generated framework to file")
}
