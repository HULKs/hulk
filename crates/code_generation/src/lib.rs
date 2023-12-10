use cyclers::generate_cyclers;
use perception_databases::generate_perception_databases;
use proc_macro2::TokenStream;
use quote::quote;
use run::{generate_replayer_struct, generate_run_function};
use source_analyzer::{cyclers::Cyclers, structs::Structs};
use structs::generate_structs;

mod accessor;
pub mod cyclers;
pub mod perception_databases;
pub mod run;
pub mod structs;
pub mod write_to_file;

pub fn generate(cyclers: &Cyclers, structs: &Structs, mode: Execution) -> TokenStream {
    let generated_cyclers = generate_cyclers(cyclers, mode);
    let generated_execution = match mode {
        Execution::None => Default::default(),
        Execution::Run => {
            let run = generate_run_function(cyclers);
            quote! {
                pub mod execution {
                    #run
                }
            }
        }
        Execution::Replay => {
            let replayer = generate_replayer_struct(cyclers);
            quote! {
                pub mod execution {
                    #replayer
                }
            }
        }
    };
    let generated_perception_databases = generate_perception_databases(cyclers);
    let generated_structs = generate_structs(structs);

    quote! {
        mod cyclers {
            #generated_cyclers
        }
        #generated_execution
        mod perception_databases {
            #generated_perception_databases
        }
        mod structs {
            #generated_structs
        }
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Execution {
    None,
    Run,
    Replay,
}
