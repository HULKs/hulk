use cyclers::generate_cyclers;
use execution::{generate_image_extractor_struct, generate_replayer_struct, generate_run_function};
use perception_databases::generate_perception_databases;
use proc_macro2::TokenStream;
use quote::quote;
use source_analyzer::{cyclers::Cyclers, structs::Structs};
use structs::generate_structs;

mod accessor;
pub mod cyclers;
pub mod execution;
pub mod perception_databases;
pub mod structs;
pub mod write_to_file;

pub fn generate(cyclers: &Cyclers, structs: &Structs, mode: ExecutionMode) -> TokenStream {
    let generated_cyclers = match mode {
        ExecutionMode::None => None,
        ExecutionMode::Run => Some(generate_cyclers(cyclers, CyclerMode::Run)),
        ExecutionMode::Replay { .. } => Some(generate_cyclers(cyclers, CyclerMode::Replay)),
    }
    .map(|cyclers| {
        quote! {
            pub mod cyclers {
                #cyclers
            }
        }
    })
    .unwrap_or_default();
    let generated_execution = match mode {
        ExecutionMode::None => Default::default(),
        ExecutionMode::Run => {
            let run = generate_run_function(cyclers);
            quote! {
                pub mod execution {
                    #run
                }
            }
        }
        ExecutionMode::Replay {
            with_communication: true,
        } => {
            let replayer = generate_replayer_struct(cyclers);
            quote! {
                pub mod execution {
                    #replayer
                }
            }
        }
        ExecutionMode::Replay {
            with_communication: false,
        } => {
            let image_extractor = generate_image_extractor_struct(cyclers);
            quote! {
                pub mod execution {
                    #image_extractor
                }
            }
        }
    };
    let generated_perception_databases = generate_perception_databases(cyclers);
    let generated_structs = generate_structs(structs);

    quote! {
        #generated_cyclers
        #generated_execution
        pub mod perception_databases {
            #generated_perception_databases
        }
        pub mod structs {
            #generated_structs
        }
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum ExecutionMode {
    None,
    Run,
    Replay { with_communication: bool },
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum CyclerMode {
    Run,
    Replay,
}
