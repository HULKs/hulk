use std::path::Path;

use proc_macro2::TokenStream;
use quote::quote;
use source_analyzer::{cycler::Cyclers, manifest::FrameworkManifest, structs::Structs};

use crate::{
    cyclers::generate_cyclers,
    perception_databases::{generate_perception_databases, generate_perception_updates},
    run::generate_run_function,
    structs::generate_structs,
};

pub fn generate_framework(cyclers: &Cyclers, structs: &Structs) -> TokenStream {
    let generated_cyclers = generate_cyclers(cyclers);
    let generated_run = generate_run_function(cyclers);
    let generated_structs = generate_structs(structs);
    let generated_perception_updates = generate_perception_updates(cyclers);
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
            #generated_perception_updates
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
