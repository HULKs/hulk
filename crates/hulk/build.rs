use code_generation::{framework::generate_framework, write_to_file::WriteToFile};
use color_eyre::eyre::{Result, WrapErr};
use proc_macro2::TokenStream;
use source_analyzer::{cycler::Cyclers, structs::Structs};

fn main() -> Result<()> {
    let generated_framework = generate()?;
    generated_framework
        .write_to_file("generated_framework.rs")
        .wrap_err("failed to write generated framework to file")
}

fn generate() -> Result<TokenStream> {
    let cyclers = Cyclers::try_from_toml("./framework.toml")?;
    let structs = Structs::try_from_cyclers(&cyclers)?;
    for cycler in &cyclers.cyclers {
        let crate_directory = "../crates";
        let module = &cycler.module;
        println!("cargo:rerun-if-changed={crate_directory}/{module}");
    }
    Ok(generate_framework(&cyclers, &structs))
}
