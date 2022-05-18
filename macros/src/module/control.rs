use module_attributes::ModuleInformation;
use proc_macro2::TokenStream;
use quote::quote;
use syn::{parse_quote, Ident, ImplItem};

use crate::module::generators::{generate_cycle_context_initializers, GenerateContextField};

use super::generators::{
    generate_change_callback_invokation, generate_main_outputs_implementation,
    generate_main_outputs_struct, generate_new_context_initializers,
};

pub fn generate_for_control(
    cycler_module: &Ident,
    mut module_information: ModuleInformation,
) -> proc_macro::TokenStream {
    let new_context_struct = generate_new_context_struct(&module_information);
    let new_context_implementation = generate_new_context_implementation(&module_information);
    let cycle_context_struct = generate_cycle_context_struct(&module_information);
    let cycle_context_implementation = generate_cycle_context_implementation(&module_information);

    let main_outputs_struct = generate_main_outputs_struct(&module_information);
    let main_outputs_implementation =
        generate_main_outputs_implementation(cycler_module, &module_information);

    let run_new_method = generate_run_new_method(&module_information);
    module_information
        .module_implementation
        .items
        .push(run_new_method);
    let run_cycle_method = generate_run_cycle_method(&module_information);
    module_information
        .module_implementation
        .items
        .push(run_cycle_method);
    let module_implementation = module_information.module_implementation;

    let output = quote! {
        #new_context_struct
        #new_context_implementation
        #cycle_context_struct
        #cycle_context_implementation
        #main_outputs_struct
        #main_outputs_implementation
        #module_implementation
    };
    output.into()
}

fn generate_new_lifetime_parameter(module_information: &ModuleInformation) -> TokenStream {
    if module_information.parameters.is_empty() {
        quote! {}
    } else {
        quote! {<'a>}
    }
}

fn generate_cycle_lifetime_parameter(module_information: &ModuleInformation) -> TokenStream {
    if module_information.additional_outputs.is_empty()
        && module_information.inputs.is_empty()
        && module_information.parameters.is_empty()
        && module_information.historic_inputs.is_empty()
        && module_information.perception_inputs.is_empty()
    {
        quote! {}
    } else {
        quote! {<'a>}
    }
}

fn generate_new_context_struct(module_information: &ModuleInformation) -> TokenStream {
    let parameters = GenerateContextField::generate(&module_information.parameters);
    let context_identifier = &module_information.new_context_identifier;
    let lifetime_parameter = generate_new_lifetime_parameter(module_information);
    quote! {
        #[allow(dead_code)]
        struct #context_identifier #lifetime_parameter {
            #parameters
        }
    }
}

fn generate_cycle_context_struct(module_information: &ModuleInformation) -> TokenStream {
    let additional_outputs = GenerateContextField::generate(&module_information.additional_outputs);
    let inputs = GenerateContextField::generate(&module_information.inputs);
    let persistent_states = GenerateContextField::generate(&module_information.persistent_states);
    let parameters = GenerateContextField::generate(&module_information.parameters);
    let historic_inputs = GenerateContextField::generate(&module_information.historic_inputs);
    let perception_inputs = GenerateContextField::generate(&module_information.perception_inputs);
    let context_identifier = &module_information.cycle_context_identifier;
    let lifetime_parameter = generate_cycle_lifetime_parameter(module_information);
    quote! {
        #[allow(dead_code)]
        struct #context_identifier #lifetime_parameter {
            #additional_outputs
            #inputs
            #persistent_states
            #parameters
            #historic_inputs
            #perception_inputs
        }
    }
}

fn generate_new_context_implementation(module_information: &ModuleInformation) -> TokenStream {
    let constructors = generate_new_context_initializers(module_information);
    let context_identifier = &module_information.new_context_identifier;
    let lifetime_parameter = generate_new_lifetime_parameter(module_information);
    quote! {
        impl <'a> #context_identifier #lifetime_parameter {
            fn new(
                configuration: &'a crate::framework::Configuration,
            ) -> Self {
                Self {
                    #constructors
                }
            }
        }
    }
}

fn generate_cycle_context_implementation(module_information: &ModuleInformation) -> TokenStream {
    let constructors = generate_cycle_context_initializers(module_information);
    let context_identifier = &module_information.cycle_context_identifier;
    let lifetime_parameter = generate_cycle_lifetime_parameter(module_information);
    quote! {
        impl <'a> #context_identifier #lifetime_parameter {
            #[allow(clippy::too_many_arguments)]
            fn new(
                cycle_start_time: std::time::SystemTime,
                this_database: &'a mut crate::control::Database,
                historic_databases: &'a crate::framework::HistoricDatabases,
                perception_databases: &'a crate::framework::PerceptionDatabases,
                configuration: &'a crate::framework::Configuration,
                subscribed_additional_outputs: &std::collections::HashSet<String>,
                cycler_configuration: &'a crate::control::Configuration,
                persistent_state: &'a mut crate::control::PersistentState,
            ) -> Self {
                Self {
                    #constructors
                }
            }
        }
    }
}

fn generate_run_new_method(module_information: &ModuleInformation) -> ImplItem {
    let module_identifier = &module_information.module_identifier;
    let context_identifier = &module_information.new_context_identifier;

    parse_quote! {
        pub fn run_new(
            configuration: &crate::framework::Configuration,
        ) -> anyhow::Result<#module_identifier> {
            Self::new(#context_identifier::new(configuration))
        }
    }
}

fn generate_run_cycle_method(module_information: &ModuleInformation) -> ImplItem {
    let change_callback_invokations = module_information
        .parameters
        .iter()
        .map(generate_change_callback_invokation);
    let context_identifier = &module_information.cycle_context_identifier;

    parse_quote! {
        #[allow(clippy::too_many_arguments)]
        pub fn run_cycle(
            &mut self,
            cycle_start_time: std::time::SystemTime,
            this_database: &mut crate::control::Database,
            historic_databases: &crate::framework::HistoricDatabases,
            perception_databases: &crate::framework::PerceptionDatabases,
            configuration: &crate::framework::Configuration,
            subscribed_additional_outputs: &std::collections::HashSet<String>,
            changed_parameters: &std::collections::HashSet<String>,
            persistent_state: &mut crate::control::PersistentState,
        ) -> anyhow::Result<()> {
            #(#change_callback_invokations)*
            let context = #context_identifier::new(
                cycle_start_time,
                this_database,
                historic_databases,
                perception_databases,
                configuration,
                subscribed_additional_outputs,
                &configuration.control,
                persistent_state,
            );
            let main_outputs = self.cycle(context)?;
            main_outputs.update(this_database);
            Ok(())
        }
    }
}
