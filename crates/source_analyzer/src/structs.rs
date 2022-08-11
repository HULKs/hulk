use std::{collections::BTreeMap, path::Path};

use anyhow::{bail, Context};
use quote::ToTokens;
use syn::Type;

use crate::{Field, Modules};

#[derive(Debug, Default)]
pub struct Structs {
    pub configuration: StructHierarchy,
    pub cycler_structs: BTreeMap<String, CyclerStructs>,
}

impl Structs {
    pub fn try_from_crates_directory<P>(crates_directory: P) -> anyhow::Result<Self>
    where
        P: AsRef<Path>,
    {
        let mut structs = Self::default();

        let modules = Modules::try_from_crates_directory(&crates_directory)
            .context("Failed to get modules")?;

        for (cycler_module, module_names) in modules.cycler_modules_to_modules.iter() {
            let cycler_structs = structs
                .cycler_structs
                .entry(cycler_module.clone())
                .or_default();

            for module_name in module_names.iter() {
                let contexts = &modules.modules[module_name].contexts;

                for field in contexts.main_outputs.iter() {
                    match field {
                        Field::MainOutput { data_type, name } => {
                            match &mut cycler_structs.main_outputs {
                                StructHierarchy::Struct { fields } => {
                                    fields.insert(
                                        name.to_string(),
                                        StructHierarchy::Field {
                                            data_type: data_type.clone(),
                                        },
                                    );
                                }
                                StructHierarchy::Field { .. } => {
                                    cycler_structs.main_outputs = StructHierarchy::Struct {
                                        fields: BTreeMap::from([(
                                            name.to_string(),
                                            StructHierarchy::Field {
                                                data_type: data_type.clone(),
                                            },
                                        )]),
                                    };
                                }
                            }
                        }
                        _ => {
                            // TODO: improve error message
                            bail!("Unexpected field in MainOutputs");
                        }
                    }
                }
                for field in contexts
                    .new_context
                    .iter()
                    .chain(contexts.cycle_context.iter())
                {
                    match field {
                        Field::AdditionalOutput { data_type, .. } => {
                            let path_segments = field
                                .get_path_segments()
                                .expect("Unexpected missing path in input field");
                            cycler_structs
                                .additional_outputs
                                .insert(&path_segments, data_type)
                                .context("Failed to insert field")?;
                        }
                        Field::Parameter { data_type, .. } => {
                            let path_segments = field
                                .get_path_segments()
                                .expect("Unexpected missing path in input field");
                            structs
                                .configuration
                                .insert(&path_segments, data_type)
                                .context("Failed to insert field")?;
                        }
                        Field::PersistentState { data_type, .. } => {
                            let path_segments = field
                                .get_path_segments()
                                .expect("Unexpected missing path in input field");
                            cycler_structs
                                .persistent_state
                                .insert(&path_segments, data_type)
                                .context("Failed to insert field")?;
                        }
                        Field::HardwareInterface { .. }
                        | Field::HistoricInput { .. }
                        | Field::OptionalInput { .. }
                        | Field::PerceptionInput { .. }
                        | Field::RequiredInput { .. } => {}
                        _ => {
                            // TODO: improve error message
                            bail!("Unexpected field in NewContext or CycleContext");
                        }
                    }
                }
            }
        }

        Ok(structs)
    }
}

#[derive(Debug, Default)]
pub struct CyclerStructs {
    pub main_outputs: StructHierarchy,
    pub additional_outputs: StructHierarchy,
    pub persistent_state: StructHierarchy,
}

#[derive(Debug)]
pub enum StructHierarchy {
    Struct {
        fields: BTreeMap<String, StructHierarchy>,
    },
    Field {
        data_type: Type,
    },
}

impl Default for StructHierarchy {
    fn default() -> Self {
        Self::Struct {
            fields: Default::default(),
        }
    }
}

impl StructHierarchy {
    pub fn insert(&mut self, path_segments: &[String], data_type: &Type) -> anyhow::Result<()> {
        match self {
            StructHierarchy::Struct { fields } => {
                let should_overwrite_children = path_segments.is_empty();
                if should_overwrite_children {
                    *self = StructHierarchy::Field {
                        data_type: data_type.clone(),
                    };
                    Ok(())
                } else {
                    let first_segment = path_segments
                        .first()
                        .expect("Unexpected empty path without overwriting children");
                    let remaining_segments: Vec<_> =
                        path_segments.iter().skip(1).cloned().collect();
                    fields
                        .entry(first_segment.clone())
                        .or_default()
                        .insert(&remaining_segments, data_type)
                }
            }
            StructHierarchy::Field {
                data_type: stored_data_type,
            } => {
                if data_type != stored_data_type {
                    bail!(
                        "Mismatched data_type of path {path_segments:?}: {} != {}",
                        data_type.to_token_stream(),
                        stored_data_type.to_token_stream()
                    );
                }
                // ignore insertion otherwise (self.data_type is responsible for defining the sub-path)
                Ok(())
            }
        }
    }
}
