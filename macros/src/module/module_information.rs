use std::mem::take;

use proc_macro_error::{abort, ResultExt};
use quote::format_ident;
use syn::{parse2, Ident, ItemImpl};

use super::attributes::{
    AdditionalOutputAttribute, HistoricInputAttribute, InputAttribute, MainOutputAttribute,
    ParameterAttribute, PerceptionInputAttribute, PersistentStateAttribute,
};

pub struct ModuleInformation {
    pub context_identifier: Ident,
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
    pub fn from_module_implementation(module_implementation: &mut ItemImpl) -> Self {
        let mut additional_outputs = Vec::new();
        let mut inputs = Vec::new();
        let mut persistent_states = Vec::new();
        let mut main_outputs = Vec::new();
        let mut parameters = Vec::new();
        let mut historic_inputs = Vec::new();
        let mut perception_inputs = Vec::new();
        let mut remaining_attributes = Vec::new();
        let attributes = take(&mut module_implementation.attrs);
        for attribute in attributes.into_iter() {
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
                "input" => inputs.push(parse2(attribute.tokens).unwrap_or_abort()),
                "persistent_state" => {
                    persistent_states.push(parse2(attribute.tokens).unwrap_or_abort())
                }
                "main_output" => main_outputs.push(parse2(attribute.tokens).unwrap_or_abort()),
                "parameter" => parameters.push(parse2(attribute.tokens).unwrap_or_abort()),
                "historic_input" => {
                    historic_inputs.push(parse2(attribute.tokens).unwrap_or_abort())
                }
                "perception_input" => {
                    perception_inputs.push(parse2(attribute.tokens).unwrap_or_abort())
                }
                _ => remaining_attributes.push(attribute),
            }
        }
        module_implementation.attrs = remaining_attributes;

        Self {
            context_identifier: format_ident!("CycleContext"),
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
