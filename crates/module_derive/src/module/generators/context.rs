use module_attributes::{
    AdditionalOutputAttribute, HistoricInputAttribute, InputAttribute, ModuleInformation,
    ParameterAttribute, PerceptionInputAttribute, PersistentStateAttribute,
};
use proc_macro2::TokenStream;
use quote::{format_ident, quote};

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
        if self.is_required {
            quote! { #name: &'a #data_type }
        } else {
            quote! { #name: &'a Option<#data_type> }
        }
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
        let cycler_database = match cycler {
            Some(cycler) => format_ident!("{}_database", cycler),
            None => format_ident!("this_database"),
        };
        let path = &self.path;
        if self.is_required {
            quote! { #name: #cycler_database.main_outputs.#(#path).*.as_ref()? }
        } else {
            quote! { #name: &#cycler_database.main_outputs.#(#path).* }
        }
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

pub fn generate_new_context_initializers(module_information: &ModuleInformation) -> TokenStream {
    let parameters = GenerateContextInitializer::generate(&module_information.parameters);
    quote! {
        #parameters
    }
}

pub fn generate_cycle_context_initializers(module_information: &ModuleInformation) -> TokenStream {
    let additional_outputs =
        GenerateContextInitializer::generate(&module_information.additional_outputs);
    let inputs = GenerateContextInitializer::generate(&module_information.inputs);
    let persistent_states =
        GenerateContextInitializer::generate(&module_information.persistent_states);
    let parameters = GenerateContextInitializer::generate(&module_information.parameters);
    let historic_inputs = GenerateContextInitializer::generate(&module_information.historic_inputs);
    let perception_inputs =
        GenerateContextInitializer::generate(&module_information.perception_inputs);
    quote! {
        #additional_outputs
        #inputs
        #persistent_states
        #parameters
        #perception_inputs
        #historic_inputs
    }
}
