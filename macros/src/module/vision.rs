use proc_macro2::TokenStream;
use quote::quote;
use syn::{parse_quote, Ident, ImplItem, ItemImpl};

use crate::module::generators::{generate_context_initializers, GenerateContextField};

use super::{
    generators::{
        generate_change_callback_invokation, generate_main_outputs_implementation,
        generate_main_outputs_struct,
    },
    module_information::ModuleInformation,
};

pub fn generate_for_vision(
    cycler_module: &Ident,
    mut module_implementation: ItemImpl,
    module_information: ModuleInformation,
) -> proc_macro::TokenStream {
    let context_struct = generate_context_struct(&module_information);
    let context_implementation = generate_context_implementation(&module_information);

    let main_outputs_struct = generate_main_outputs_struct(&module_information);
    let main_outputs_implementation =
        generate_main_outputs_implementation(cycler_module, &module_information);

    let run_cycle_method = generate_run_cycle_method(&module_information);
    module_implementation.items.push(run_cycle_method);

    let output = quote! {
        use crate::types::Image422;
        #context_struct
        #context_implementation
        #main_outputs_struct
        #main_outputs_implementation
        #module_implementation
    };
    output.into()
}

fn generate_context_struct(module_information: &ModuleInformation) -> TokenStream {
    let additional_outputs = GenerateContextField::generate(&module_information.additional_outputs);
    let inputs = GenerateContextField::generate(&module_information.inputs);
    let parameters = GenerateContextField::generate(&module_information.parameters);
    let perception_inputs = GenerateContextField::generate(&module_information.perception_inputs);
    let context_identifier = &module_information.context_identifier;
    quote! {
        struct #context_identifier <'a> {
            image: &'a Image422,
            camera_position: crate::types::CameraPosition,
            #additional_outputs
            #inputs
            #parameters
            #perception_inputs
        }
    }
}

fn generate_context_implementation(module_information: &ModuleInformation) -> TokenStream {
    let constructors = generate_context_initializers(module_information);
    let context_identifier = &module_information.context_identifier;
    quote! {
        impl <'a> #context_identifier <'a> {
            #[allow(clippy::too_many_arguments)]
            fn new(
                image: &'a Image422,
                camera_position: crate::types::CameraPosition,
                this_database: &'a mut crate::vision::Database,
                control_database: &'a crate::control::Database,
                configuration: &'a crate::framework::Configuration,
                cycler_configuration: &'a crate::vision::Configuration,
                subscribed_additional_outputs: &std::collections::HashSet<String>,
            ) -> Self {
                Self {
                    image,
                    camera_position,
                    #constructors
                }
            }
        }
    }
}

fn generate_run_cycle_method(module_information: &ModuleInformation) -> ImplItem {
    let change_callback_invokations = module_information
        .parameters
        .iter()
        .map(generate_change_callback_invokation);
    let context_identifier = &module_information.context_identifier;

    parse_quote! {
        #[allow(clippy::too_many_arguments)]
        pub fn run_cycle(
            &mut self,
            image: &Image422,
            camera_position: crate::types::CameraPosition,
            this_database: &mut crate::vision::Database,
            control_database: &crate::control::Database,
            configuration: &crate::framework::Configuration,
            cycler_configuration: &crate::vision::Configuration,
            subscribed_additional_outputs: &std::collections::HashSet<String>,
            changed_parameters: &std::collections::HashSet<String>,
        ) -> anyhow::Result<()> {
            #(#change_callback_invokations)*
            let inputs = #context_identifier::new(
                image,
                camera_position,
                this_database,
                control_database,
                configuration,
                cycler_configuration,
                subscribed_additional_outputs,
            );
            let main_outputs = self.cycle(inputs)?;
            main_outputs.update(this_database);
            Ok(())
        }
    }
}
