use anyhow::{bail, Context};
use build_script_helpers::write_token_stream;
use code_generation::{
    cycler::{generate_cyclers, get_cyclers},
    run::generate_run,
};
use quote::quote;
use source_analyzer::{
    cycler_crates_from_crates_directory, CyclerInstances, CyclerTypes, Field, Modules,
};

mod code_generation;

fn main() -> anyhow::Result<()> {
    for crate_directory in cycler_crates_from_crates_directory("..")
        .context("Failed to get cycler crate directories from crates directory")?
    {
        println!("cargo:rerun-if-changed={}", crate_directory.display());
    }

    let cycler_instances = CyclerInstances::try_from_crates_directory("..")
        .context("Failed to get cycler instances from crates directory")?;
    let mut modules = Modules::try_from_crates_directory("..")
        .context("Failed to get modules from crates directory")?;
    modules.sort().context("Failed to sort modules")?;
    let cycler_types = CyclerTypes::try_from_crates_directory("..")
        .context("Failed to get perception cycler instances from crates directory")?;

    for module_names in modules.cycler_modules_to_modules.values() {
        let first_module_name = match module_names.first() {
            Some(first_module_name) => first_module_name,
            None => continue,
        };
        for field in modules.modules[first_module_name]
            .contexts
            .cycle_context
            .iter()
        {
            match field {
                Field::HistoricInput { name, .. } => bail!(
                    "Unexpected historic input for first module `{first_module_name}` in `{}` for `{name}` in cycle context",
                    modules.modules[first_module_name].cycler_module
                ),
                Field::Input { name, .. } => bail!(
                    "Unexpected optional input for first module `{first_module_name}` in `{}` for `{name}` in cycle context",
                    modules.modules[first_module_name].cycler_module
                ),
                Field::PerceptionInput { name, .. } => bail!(
                    "Unexpected perception input for first module `{first_module_name}` in `{}` for `{name}` in cycle context",
                    modules.modules[first_module_name].cycler_module
                ),
                Field::RequiredInput { name, .. } => bail!(
                    "Unexpected required input for first module `{first_module_name}` in `{}` for `{name}` in cycle context",
                    modules.modules[first_module_name].cycler_module
                ),
                _ => {}
            }
        }
    }

    let cyclers = get_cyclers(&cycler_instances, &modules, &cycler_types);

    let cyclers_token_stream = generate_cyclers(&cyclers).context("Failed to generate cyclers")?;
    let runtime_token_stream = generate_run(&cyclers);

    write_token_stream(
        "cyclers.rs",
        quote! {
            #cyclers_token_stream
            #runtime_token_stream
        },
    )
    .context("Failed to write cyclers")?;

    Ok(())
}
