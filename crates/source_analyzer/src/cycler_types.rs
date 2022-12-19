use std::{collections::HashMap, path::Path};

use color_eyre::{eyre::WrapErr, Result};

use crate::{CyclerInstances, Field, Modules};

#[derive(Debug)]
pub struct CyclerTypes {
    pub cycler_modules_to_cycler_types: HashMap<String, CyclerType>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CyclerType {
    Perception,
    RealTime,
}

impl CyclerTypes {
    pub fn try_from_crates_directory(crates_directory: impl AsRef<Path>) -> Result<Self> {
        let cycler_instances = CyclerInstances::try_from_crates_directory(&crates_directory)
            .wrap_err("failed to get cycler_instances")?;
        let modules = Modules::try_from_crates_directory(&crates_directory)
            .wrap_err("failed to get modules")?;

        Ok(Self {
            cycler_modules_to_cycler_types: cycler_instances
                .modules_to_instances
                .keys()
                .map(|cycler_module_name| {
                    let at_least_one_module_uses_this_cycler_module_via_perception_input =
                        modules.modules.values().any(|module| {
                            module
                                .contexts
                                .creation_context
                                .iter()
                                .chain(module.contexts.cycle_context.iter())
                                .any(|field| match field {
                                    Field::PerceptionInput {
                                        cycler_instance, ..
                                    } if &cycler_instances.instances_to_modules
                                            [cycler_instance] // TODO: wenn der Benutzer einen Cycler angibt, der nicht existiert, panikiert hier was
                                            == cycler_module_name =>
                                    {
                                        true
                                    }
                                    _ => false,
                                })
                        });
                    (
                        cycler_module_name.clone(),
                        match at_least_one_module_uses_this_cycler_module_via_perception_input {
                            true => CyclerType::Perception,
                            false => CyclerType::RealTime,
                        },
                    )
                })
                .collect(),
        })
    }
}
