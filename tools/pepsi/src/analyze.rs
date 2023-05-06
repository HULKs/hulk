use std::path::PathBuf;

use clap::Subcommand;
use color_eyre::{eyre::WrapErr, Result};

use source_analyzer::{
    contexts::Contexts, cycler::Cyclers, manifest::FrameworkManifest, node::parse_rust_file,
    structs::Structs,
};

#[derive(Subcommand)]
#[allow(clippy::enum_variant_names)]
pub enum Arguments {
    DumpContexts {
        /// File path to a Rust file containing a module with context structs
        file_path: PathBuf,
    },
    DumpCyclers {
        /// File path to a framework manifest
        manifest_path: PathBuf,
    },
    DumpStructs {
        /// File path to a framework manifest
        manifest_path: PathBuf,
    },
}

pub async fn analyze(arguments: Arguments) -> Result<()> {
    match arguments {
        Arguments::DumpContexts { file_path } => {
            let file = parse_rust_file(file_path).wrap_err("failed to parse rust file")?;
            let context =
                Contexts::try_from_file(&file).wrap_err("failed to get contexts from rust file")?;
            println!("{context}");
        }
        Arguments::DumpCyclers { manifest_path } => {
            let manifest = FrameworkManifest::try_from_toml(&manifest_path)?;
            let mut cyclers =
                Cyclers::try_from_manifest(manifest, manifest_path.parent().unwrap().join(".."))?;
            cyclers.sort_nodes()?;
            println!("{cyclers}");
        }
        Arguments::DumpStructs { manifest_path } => {
            let manifest = FrameworkManifest::try_from_toml(&manifest_path)?;
            let cyclers =
                Cyclers::try_from_manifest(manifest, manifest_path.parent().unwrap().join(".."))?;
            let structs = Structs::try_from_cyclers(&cyclers)?;
            println!("{structs:#?}");
        }
    }

    Ok(())
}
