use std::path::Path;

use cyclers::generate_cyclers;
use perception_databases::generate_perception_databases;
use proc_macro2::TokenStream;
use quote::quote;
use run::generate_run_function;
use source_analyzer::{cycler::Cyclers, manifest::FrameworkManifest, structs::Structs};
use structs::generate_structs;

mod accessor;
pub mod cyclers;
pub mod perception_databases;
pub mod run;
pub mod structs;
pub mod write_to_file;

pub fn generate(cyclers: &Cyclers, structs: &Structs) -> TokenStream {
    let generated_cyclers = generate_cyclers(cyclers);
    let generated_run = generate_run_function(cyclers);
    let generated_structs = generate_structs(structs);
    let generated_perception_databases = generate_perception_databases(cyclers);

    quote! {
        mod cyclers {
            #generated_cyclers
        }
        pub mod run {
            #generated_run
        }
        mod structs {
            #generated_structs
        }
        mod perception_databases {
            #generated_perception_databases
        }
    }
}

pub fn collect_watch_paths(manifest: &FrameworkManifest) -> impl Iterator<Item = &Path> {
    manifest.cyclers.iter().flat_map(|cycler| {
        cycler
            .setup_nodes
            .iter()
            .chain(cycler.nodes.iter())
            .map(|specification| specification.path.as_path())
    })
}
