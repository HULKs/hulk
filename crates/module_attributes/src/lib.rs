use convert_case::{Case, Casing};
use proc_macro_error::{abort, ResultExt};
use quote::format_ident;
use syn::{parse2, Ident, ItemImpl, Type, TypePath};

pub use crate::attributes::{
    AdditionalOutputAttribute, HistoricInputAttribute, InputAttribute, MainOutputAttribute,
    ParameterAttribute, PerceptionInputAttribute, PersistentStateAttribute,
};

mod attributes;

fn to_snake_case(identifier: &TypePath) -> Ident {
    format_ident!(
        "{}",
        &match identifier.path.get_ident() {
            Some(identifier) => identifier,
            None => &identifier.path.segments.last().unwrap().ident,
        }
        .to_string()
        .to_case(Case::Snake)
    )
}

#[derive(Debug)]
pub struct ModuleInformation {
    pub module_implementation: ItemImpl,
    pub module_identifier: Ident,
    pub module_snake_case_identifier: Ident,
    pub new_context_identifier: Ident,
    pub cycle_context_identifier: Ident,
    pub main_outputs_identifier: Ident,
    pub additional_outputs: Vec<AdditionalOutputAttribute>,
    pub inputs: Vec<InputAttribute>,
    pub persistent_states: Vec<PersistentStateAttribute>,
    pub main_outputs: Vec<MainOutputAttribute>,
    pub parameters: Vec<ParameterAttribute>,
    pub historic_inputs: Vec<HistoricInputAttribute>,
    pub perception_inputs: Vec<PerceptionInputAttribute>,
}

impl ModuleInformation {
    pub fn from_module_implementation(mut module_implementation: ItemImpl) -> Self {
        let mut additional_outputs = Vec::new();
        let mut historic_inputs = Vec::new();
        let mut inputs = Vec::new();
        let mut main_outputs = Vec::new();
        let mut parameters = Vec::new();
        let mut perception_inputs = Vec::new();
        let mut persistent_states = Vec::new();
        let mut remaining_attributes = Vec::new();
        for attribute in module_implementation.attrs.into_iter() {
            let attribute_name = match attribute.path.get_ident() {
                Some(identifier) => identifier,
                None => abort!(
                    attribute.path,
                    "expected single segment path without arguments"
                ),
            };
            match attribute_name.to_string().as_str() {
                "additional_output" => {
                    additional_outputs.push(parse2(attribute.tokens).unwrap_or_abort())
                }
                "historic_input" => {
                    historic_inputs.push(parse2(attribute.tokens).unwrap_or_abort())
                }
                "input" => inputs.push(parse2(attribute.tokens).unwrap_or_abort()),
                "main_output" => main_outputs.push(parse2(attribute.tokens).unwrap_or_abort()),
                "parameter" => parameters.push(parse2(attribute.tokens).unwrap_or_abort()),
                "perception_input" => {
                    perception_inputs.push(parse2(attribute.tokens).unwrap_or_abort())
                }
                "persistent_state" => {
                    persistent_states.push(parse2(attribute.tokens).unwrap_or_abort())
                }
                _ => remaining_attributes.push(attribute),
            }
        }
        module_implementation.attrs = remaining_attributes;
        let type_path = match *module_implementation.self_ty {
            Type::Path(ref type_path) => type_path,
            _ => abort!(module_implementation.self_ty, "expected TypePath"),
        };
        let module_snake_case_identifier = to_snake_case(type_path);
        let module_identifier = match type_path.path.get_ident() {
            Some(identifier) => identifier.clone(),
            None => abort!(
                type_path.path,
                "expected single segment path without arguments"
            ),
        };

        Self {
            module_implementation,
            module_identifier,
            module_snake_case_identifier,
            new_context_identifier: format_ident!("NewContext"),
            cycle_context_identifier: format_ident!("CycleContext"),
            main_outputs_identifier: format_ident!("MainOutputs"),
            additional_outputs,
            inputs,
            persistent_states,
            main_outputs,
            parameters,
            historic_inputs,
            perception_inputs,
        }
    }
}
