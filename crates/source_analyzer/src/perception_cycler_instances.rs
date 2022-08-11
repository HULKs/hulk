use std::{collections::HashSet, path::Path};

use anyhow::Context;

use crate::{Field, Modules};

pub struct PerceptionCyclersInstances {
    pub perception_cycler_instances: HashSet<String>,
}

impl PerceptionCyclersInstances {
    pub fn try_from_crates_directory<P>(crates_directory: P) -> anyhow::Result<Self>
    where
        P: AsRef<Path>,
    {
        let modules = Modules::try_from_crates_directory(&crates_directory)
            .context("Failed to get modules")?;

        let perception_cycler_instances = modules
            .modules
            .values()
            .map(|module| {
                module
                    .contexts
                    .new_context
                    .iter()
                    .chain(module.contexts.cycle_context.iter())
                    .filter_map(|field| match field {
                        Field::PerceptionInput {
                            cycler_instance, ..
                        } => Some(
                            cycler_instance
                                .token()
                                .to_string()
                                .trim_matches('"')
                                .to_string(),
                        ),
                        _ => None,
                    })
            })
            .flatten()
            .collect();

        Ok(Self {
            perception_cycler_instances,
        })
    }
}
