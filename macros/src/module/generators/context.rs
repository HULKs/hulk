use proc_macro2::TokenStream;
use quote::{format_ident, quote};

use crate::module::{
    attributes::{
        AdditionalOutputAttribute, HistoricInputAttribute, InputAttribute, ParameterAttribute,
        PerceptionInputAttribute, PersistentStateAttribute,
    },
    module_information::ModuleInformation,
};

pub trait GenerateContextField {
    fn generate(&self) -> TokenStream;
}

impl GenerateContextField for AdditionalOutputAttribute {
    fn generate(&self) -> TokenStream {
        let name = &self.name;
        let data_type = &self.data_type;
        quote! { #name: crate::framework::AdditionalOutput<'a, #data_type> }
    }
}

impl GenerateContextField for InputAttribute {
    fn generate(&self) -> TokenStream {
        let name = &self.name;
        let data_type = &self.data_type;
        quote! { #name: &'a Option<#data_type> }
    }
}

impl GenerateContextField for PersistentStateAttribute {
    fn generate(&self) -> TokenStream {
        let name = &self.name;
        let data_type = &self.data_type;
        quote! { #name: &'a mut #data_type }
    }
}

impl GenerateContextField for ParameterAttribute {
    fn generate(&self) -> TokenStream {
        let name = &self.name;
        let data_type = &self.data_type;
        quote! { #name: &'a #data_type }
    }
}

impl GenerateContextField for PerceptionInputAttribute {
    fn generate(&self) -> TokenStream {
        let name = &self.name;
        let data_type = &self.data_type;
        quote! { #name: crate::framework::PerceptionDataType<'a, Option<#data_type>> }
    }
}

impl GenerateContextField for HistoricInputAttribute {
    fn generate(&self) -> TokenStream {
        let name = &self.name;
        let data_type = &self.data_type;
        quote! { #name: crate::framework::HistoricDataType<'a, Option<#data_type>> }
    }
}

impl<T> GenerateContextField for Vec<T>
where
    T: GenerateContextField,
{
    fn generate(&self) -> TokenStream {
        let fields = self
            .iter()
            .map(GenerateContextField::generate)
            .collect::<Vec<_>>();
        if fields.is_empty() {
            TokenStream::new()
        } else {
            quote! {#(#fields),*,}
        }
    }
}

trait GenerateContextInitializer {
    fn generate(&self) -> TokenStream;
}

impl GenerateContextInitializer for AdditionalOutputAttribute {
    fn generate(&self) -> TokenStream {
        let name = &self.name;
        let path = &self.path;
        let field_path = quote! { #(#path).* }.to_string();
        quote! {
            #name: crate::framework::AdditionalOutput::new(
                subscribed_additional_outputs.contains(#field_path),
                &mut this_database.additional_outputs.#(#path).*,
            )
        }
    }
}

impl GenerateContextInitializer for InputAttribute {
    fn generate(&self) -> TokenStream {
        let name = &self.name;
        let cycler = &self.cycler;
        let cycler_database = format_ident!("{}_database", cycler);
        let path = &self.path;
        quote! { #name: &#cycler_database.main_outputs.#(#path).* }
    }
}

impl GenerateContextInitializer for PersistentStateAttribute {
    fn generate(&self) -> TokenStream {
        let name = &self.name;
        let path = &self.path;
        quote! { #name: &mut persistent_state.#(#path).* }
    }
}

impl GenerateContextInitializer for ParameterAttribute {
    fn generate(&self) -> TokenStream {
        let name = &self.name;
        let path = &self.path;
        if self.path_is_relative_to_cycler {
            quote! {
                #name: &cycler_configuration.#(#path).*
            }
        } else {
            quote! {
                #name: &configuration.#(#path).*
            }
        }
    }
}

impl GenerateContextInitializer for HistoricInputAttribute {
    fn generate(&self) -> TokenStream {
        let name = &self.name;
        let path = &self.path;
        quote! { #name: crate::framework::HistoricDataType::new(
            cycle_start_time,
            historic_databases,
            &this_database.main_outputs.#(#path).*,
            |(timestamp, database)| {
                (
                    *timestamp,
                    &database.main_outputs.#(#path).*,
                )
            },
        ) }
    }
}

impl GenerateContextInitializer for PerceptionInputAttribute {
    fn generate(&self) -> TokenStream {
        let name = &self.name;
        let cycler_name = &self.cycler;
        let path = &self.path;
        quote! { #name: crate::framework::PerceptionDataType::new(
            perception_databases,
            |(timestamp, databases)| {
                (
                    *timestamp,
                    databases
                        .#cycler_name
                        .iter()
                        .map(|database| &database.#(#path).*)
                        .collect(),
                )
            },
        ) }
    }
}

impl<T> GenerateContextInitializer for Vec<T>
where
    T: GenerateContextInitializer,
{
    fn generate(&self) -> TokenStream {
        let fields = self
            .iter()
            .map(GenerateContextInitializer::generate)
            .collect::<Vec<_>>();
        if fields.is_empty() {
            TokenStream::new()
        } else {
            quote! {#(#fields),*,}
        }
    }
}

pub fn generate_context_initializers(cycle_method: &ModuleInformation) -> TokenStream {
    let additional_outputs = GenerateContextInitializer::generate(&cycle_method.additional_outputs);
    let inputs = GenerateContextInitializer::generate(&cycle_method.inputs);
    let persistent_states = GenerateContextInitializer::generate(&cycle_method.persistent_states);
    let parameters = GenerateContextInitializer::generate(&cycle_method.parameters);
    let historic_inputs = GenerateContextInitializer::generate(&cycle_method.historic_inputs);
    let perception_inputs = GenerateContextInitializer::generate(&cycle_method.perception_inputs);
    quote! {
        #additional_outputs
        #inputs
        #persistent_states
        #parameters
        #perception_inputs
        #historic_inputs
    }
}
