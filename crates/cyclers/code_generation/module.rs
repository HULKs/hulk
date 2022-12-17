use color_eyre::{
    eyre::{bail, WrapErr},
    Result,
};
use convert_case::{Case, Casing};
use proc_macro2::{Ident, TokenStream};
use quote::{format_ident, quote};
use source_analyzer::{CyclerInstances, Field};

use super::{accessor::path_to_accessor_token_stream, reference_type::ReferenceType};

pub struct Module<'a> {
    pub cycler_instances: &'a CyclerInstances,
    pub module_name: &'a str,
    pub module: &'a source_analyzer::Module,
}

impl Module<'_> {
    pub fn get_identifier(&self) -> Ident {
        format_ident!("{}", self.module_name)
    }

    pub fn get_identifier_snake_case(&self) -> Ident {
        format_ident!("{}", self.module_name.to_case(Case::Snake))
    }

    pub fn get_path_segments(&self) -> Vec<Ident> {
        self.module
            .path_segments
            .iter()
            .map(|segment| format_ident!("{}", segment))
            .collect()
    }

    pub fn get_field(&self) -> TokenStream {
        let module_name_identifier_snake_case = self.get_identifier_snake_case();
        let module_name_identifier = self.get_identifier();
        let path_segments = self.get_path_segments();
        let cycler_module_name_identifier = format_ident!("{}", self.module.cycler_module);

        quote! {
            #module_name_identifier_snake_case:
                #cycler_module_name_identifier::#(#path_segments::)*#module_name_identifier
        }
    }

    pub fn get_initializer_field_initializers(&self) -> Result<Vec<TokenStream>> {
        let cycler_module_name_identifier = format_ident!("{}", self.module.cycler_module);
        self.module
            .contexts
            .creation_context
            .iter()
            .map(|field| match field {
                Field::AdditionalOutput { name, .. } => {
                    bail!("unexpected additional output field `{name}` in new context")
                }
                Field::CyclerInstance { name } => Ok(quote! {
                    #name: instance
                }),
                Field::HardwareInterface { name } => Ok(quote! {
                    #name: &hardware_interface
                }),
                Field::HistoricInput { name, .. } => {
                    bail!("unexpected historic input field `{name}` in new context")
                }
                Field::Input { name, .. } => {
                    bail!("unexpected optional input field `{name}` in new context")
                }
                Field::MainOutput { name, .. } => {
                    bail!("unexpected main output field `{name}` in new context")
                }
                Field::Parameter { name, path, .. } => {
                    let accessor = path_to_accessor_token_stream(
                        quote! { configuration },
                        path,
                        ReferenceType::Immutable,
                        quote! { instance },
                        quote! { #cycler_module_name_identifier::CyclerInstance:: },
                        &self.cycler_instances.modules_to_instances[&self.module.cycler_module],
                    );
                    Ok(quote! {
                        #name: #accessor
                    })
                }
                Field::PerceptionInput { name, .. } => {
                    bail!("unexpected perception input field `{name}` in new context")
                }
                Field::PersistentState { name, path, .. } => {
                    let accessor = path_to_accessor_token_stream(
                        quote! { persistent_state },
                        path,
                        ReferenceType::Mutable,
                        quote! { instance },
                        quote! { #cycler_module_name_identifier::CyclerInstance:: },
                        &self.cycler_instances.modules_to_instances[&self.module.cycler_module],
                    );
                    Ok(quote! {
                        #name: #accessor
                    })
                }
                Field::RequiredInput { name, .. } => {
                    bail!("unexpected required input field `{name}` in new context")
                }
            })
            .collect()
    }

    pub fn get_initializer(&self) -> Result<TokenStream> {
        let module_name_identifier_snake_case = self.get_identifier_snake_case();
        let module_name_identifier = self.get_identifier();
        let path_segments = self.get_path_segments();
        let cycler_module_name_identifier = format_ident!("{}", self.module.cycler_module);
        let field_initializers = self
            .get_initializer_field_initializers()
            .wrap_err("failed to generate field initializers")?;
        let error_message = format!("failed to create module `{}`", self.module_name);

        Ok(quote! {
            let #module_name_identifier_snake_case = #cycler_module_name_identifier::#(#path_segments::)*#module_name_identifier::new(
                #cycler_module_name_identifier::#(#path_segments::)*CreationContext {
                    #(#field_initializers,)*
                },
            )
            .wrap_err(#error_message)?;
        })
    }

    pub fn get_required_inputs_are_some(&self) -> Option<TokenStream> {
        let cycler_module_name_identifier = format_ident!("{}", self.module.cycler_module);
        let required_inputs_are_some: Vec<_> = self
            .module
            .contexts
            .cycle_context
            .iter()
            .filter_map(|field| match field {
                Field::RequiredInput {
                    path,
                    cycler_instance,
                    ..
                } => {
                    let database_prefix = match cycler_instance {
                        Some(cycler_instance) => {
                            let identifier =
                                format_ident!("{}_database", cycler_instance.to_case(Case::Snake));
                            quote! { #identifier.main_outputs }
                        }
                        None => {
                            quote! { own_database_reference.main_outputs }
                        }
                    };
                    let accessor = path_to_accessor_token_stream(
                        database_prefix,
                        path,
                        ReferenceType::Immutable,
                        quote! { self.instance },
                        quote! { #cycler_module_name_identifier::CyclerInstance:: },
                        &self.cycler_instances.modules_to_instances[&self.module.cycler_module],
                    );
                    // TODO: check if required input actually has at least one optional
                    Some(quote! {
                        #accessor .is_some()
                    })
                }
                _ => None,
            })
            .collect();
        match required_inputs_are_some.is_empty() {
            true => None,
            false => Some(quote! {
                #(#required_inputs_are_some)&&*
            }),
        }
    }

    pub fn get_execution_field_initializers(&self) -> Result<Vec<TokenStream>> {
        let cycler_module_name_identifier = format_ident!("{}", self.module.cycler_module);
        self.module
            .contexts
            .cycle_context
            .iter()
            .map(|field| match field {
                Field::AdditionalOutput { name, path, .. } => {
                    let accessor = path_to_accessor_token_stream(
                        quote! { own_database_reference.additional_outputs },
                        path,
                        ReferenceType::Mutable,
                        quote! { self.instance },
                        quote! { #cycler_module_name_identifier::CyclerInstance:: },
                        &self.cycler_instances.modules_to_instances[&self.module.cycler_module],
                    );
                    // TODO: is_subscribed
                    Ok(quote! {
                        #name: framework::AdditionalOutput::new(
                            false,
                            #accessor,
                        )
                    })
                }
                Field::CyclerInstance { name } => Ok(quote! {
                    #name: self.instance
                }),
                Field::HardwareInterface { name } => Ok(quote! {
                    #name: &self.hardware_interface
                }),
                Field::HistoricInput { name, path, .. } => {
                    let now_accessor = path_to_accessor_token_stream(
                        quote! { own_database_reference.main_outputs },
                        path,
                        ReferenceType::Immutable,
                        quote! { self.instance },
                        quote! { #cycler_module_name_identifier::CyclerInstance:: },
                        &self.cycler_instances.modules_to_instances[&self.module.cycler_module],
                    );
                    let historic_accessor = path_to_accessor_token_stream(
                        quote! { database },
                        path,
                        ReferenceType::Immutable,
                        quote! { self.instance },
                        quote! { #cycler_module_name_identifier::CyclerInstance:: },
                        &self.cycler_instances.modules_to_instances[&self.module.cycler_module],
                    );
                    Ok(quote! {
                        #name: [(now, #now_accessor)]
                            .into_iter()
                            .chain(
                                self
                                    .historic_databases
                                    .databases
                                    .iter()
                                    .map(|(system_time, database)| (
                                        *system_time,
                                        #historic_accessor,
                                    ))
                            )
                            .collect::<std::collections::BTreeMap<_, _>>()
                            .into()
                    })
                }
                Field::Input {
                    cycler_instance,
                    name,
                    path,
                    ..
                } => {
                    let database_prefix = match cycler_instance {
                        Some(cycler_instance) => {
                            let identifier =
                                format_ident!("{}_database", cycler_instance.to_case(Case::Snake));
                            quote! { #identifier.main_outputs }
                        }
                        None => {
                            quote! { own_database_reference.main_outputs }
                        }
                    };
                    let accessor = path_to_accessor_token_stream(
                        database_prefix,
                        path,
                        ReferenceType::Immutable,
                        quote! { self.instance },
                        quote! { #cycler_module_name_identifier::CyclerInstance:: },
                        &self.cycler_instances.modules_to_instances[&self.module.cycler_module],
                    );
                    Ok(quote! {
                        #name: #accessor
                    })
                }
                Field::MainOutput { name, .. } => {
                    bail!("unexpected main output field `{name}` in cycle context")
                }
                Field::Parameter { name, path, .. } => {
                    let accessor = path_to_accessor_token_stream(
                        quote! { configuration },
                        path,
                        ReferenceType::Immutable,
                        quote! { self.instance },
                        quote! { #cycler_module_name_identifier::CyclerInstance:: },
                        &self.cycler_instances.modules_to_instances[&self.module.cycler_module],
                    );
                    Ok(quote! {
                        #name: #accessor
                    })
                }
                Field::PerceptionInput {
                    cycler_instance,
                    name,
                    path,
                    ..
                } => {
                    let cycler_instance_identifier =
                        format_ident!("{}", cycler_instance.to_case(Case::Snake));
                    let accessor = path_to_accessor_token_stream(
                        quote! { database },
                        path,
                        ReferenceType::Immutable,
                        quote! { self.instance },
                        quote! { #cycler_module_name_identifier::CyclerInstance:: },
                        &self.cycler_instances.modules_to_instances[&self.module.cycler_module],
                    );
                    Ok(quote! {
                        #name: framework::PerceptionInput {
                            persistent: self
                                .perception_databases
                                .persistent()
                                .map(|(system_time, databases)| (
                                    *system_time,
                                    databases
                                        .#cycler_instance_identifier
                                        .iter()
                                        .map(|database| #accessor)
                                        .collect()
                                    ,
                                ))
                                .collect(),
                            temporary: self
                                .perception_databases
                                .temporary()
                                .map(|(system_time, databases)| (
                                    *system_time,
                                    databases
                                        .#cycler_instance_identifier
                                        .iter()
                                        .map(|database| #accessor)
                                        .collect()
                                    ,
                                ))
                                .collect(),
                        }
                    })
                }
                Field::PersistentState { name, path, .. } => {
                    let accessor = path_to_accessor_token_stream(
                        quote! { self.persistent_state },
                        path,
                        ReferenceType::Mutable,
                        quote! { self.instance },
                        quote! { #cycler_module_name_identifier::CyclerInstance:: },
                        &self.cycler_instances.modules_to_instances[&self.module.cycler_module],
                    );
                    Ok(quote! {
                        #name: #accessor
                    })
                }
                Field::RequiredInput {
                    cycler_instance,
                    name,
                    path,
                    ..
                } => {
                    let database_prefix = match cycler_instance {
                        Some(cycler_instance) => {
                            let identifier =
                                format_ident!("{}_database", cycler_instance.to_case(Case::Snake));
                            quote! { #identifier.main_outputs }
                        }
                        None => {
                            quote! { own_database_reference.main_outputs }
                        }
                    };
                    let accessor = path_to_accessor_token_stream(
                        database_prefix,
                        path,
                        ReferenceType::Immutable,
                        quote! { self.instance },
                        quote! { #cycler_module_name_identifier::CyclerInstance:: },
                        &self.cycler_instances.modules_to_instances[&self.module.cycler_module],
                    );
                    Ok(quote! {
                        #name: #accessor .unwrap()
                    })
                }
            })
            .collect()
    }

    pub fn get_main_output_setters_from_cycle_result(&self) -> Vec<TokenStream> {
        self.module
            .contexts
            .main_outputs
            .iter()
            .filter_map(|field| match field {
                Field::MainOutput { name, .. } => Some(quote! {
                    own_database_reference.main_outputs.#name = main_outputs.#name.value;
                }),
                _ => None,
            })
            .collect()
    }

    pub fn get_main_output_setters_from_default(&self) -> Vec<TokenStream> {
        self.module
            .contexts
            .main_outputs
            .iter()
            .filter_map(|field| match field {
                Field::MainOutput { name, .. } => Some(quote! {
                    own_database_reference.main_outputs.#name = Default::default();
                }),
                _ => None,
            })
            .collect()
    }

    pub fn get_execution(&self) -> Result<TokenStream> {
        let module_name_identifier_snake_case = self.get_identifier_snake_case();
        let path_segments = self.get_path_segments();
        let cycler_module_name_identifier = format_ident!("{}", self.module.cycler_module);
        let required_inputs_are_some = self.get_required_inputs_are_some();
        let field_initializers = self
            .get_execution_field_initializers()
            .wrap_err("failed to generate field initializers")?;
        let main_output_setters_from_cycle_result =
            self.get_main_output_setters_from_cycle_result();
        let main_output_setters_from_default = self.get_main_output_setters_from_default();
        let error_message = format!("failed to execute cycle of module `{}`", self.module_name);
        let module_execution = quote! {
            let main_outputs = self.#module_name_identifier_snake_case.cycle(
                #cycler_module_name_identifier::#(#path_segments::)*CycleContext {
                    #(#field_initializers,)*
                },
            )
            .wrap_err(#error_message)?;
            #(#main_output_setters_from_cycle_result)*
        };

        match required_inputs_are_some {
            Some(required_inputs_are_some) => Ok(quote! {
                if #required_inputs_are_some {
                    #module_execution
                } else {
                    #(#main_output_setters_from_default)*
                }
            }),
            None => Ok(quote! {
                {
                    #module_execution
                }
            }),
        }
    }
}
