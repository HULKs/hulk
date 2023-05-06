use code_generation::{
    framework::{collect_watch_paths, generate_framework},
    write_to_file::WriteToFile,
};
use color_eyre::eyre::{Result, WrapErr};
use source_analyzer::{cycler::Cyclers, manifest::FrameworkManifest, structs::Structs};

fn main() -> Result<()> {
    println!("cargo:rerun-if-changed=framework.toml");
    let manifest = FrameworkManifest::try_from_toml("framework.toml")?;
    let root = "..";

    for path in collect_watch_paths(&manifest) {
        println!("cargo:rerun-if-changed={root}/{}", path.display());
    }

    let mut cyclers = Cyclers::try_from_manifest(manifest, root)?;
    cyclers.sort_nodes()?;
    let structs = Structs::try_from_cyclers(&cyclers)?;
    generate_framework(&cyclers, &structs)
        .write_to_file("generated_framework.rs")
        .wrap_err("failed to write generated framework to file")
}
