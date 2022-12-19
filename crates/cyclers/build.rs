use build_script_helpers::write_token_stream;
use code_generation::{
    cycler::{generate_cyclers, get_cyclers},
    run::generate_run,
};
use color_eyre::{
    eyre::{bail, WrapErr},
    Result,
};
use quote::quote;
use source_analyzer::{
    cycler_crates_from_crates_directory, CyclerInstances, CyclerTypes, Field, Nodes,
};

mod code_generation;

fn main() -> Result<()> {
    for crate_directory in cycler_crates_from_crates_directory("..")
        .wrap_err("failed to get cycler crate directories from crates directory")?
    {
        println!("cargo:rerun-if-changed={}", crate_directory.display());
    }

    let cycler_instances = CyclerInstances::try_from_crates_directory("..")
        .wrap_err("failed to get cycler instances from crates directory")?;
    let mut nodes = Nodes::try_from_crates_directory("..")
        .wrap_err("failed to get nodes from crates directory")?;
    nodes.sort().wrap_err("failed to sort nodes")?;
    let cycler_types = CyclerTypes::try_from_crates_directory("..")
        .wrap_err("failed to get perception cycler instances from crates directory")?;

    for node_names in nodes.cycler_modules_to_nodes.values() {
        let first_node_name = match node_names.first() {
            Some(first_node_name) => first_node_name,
            None => continue,
        };
        for field in nodes.nodes[first_node_name].contexts.cycle_context.iter() {
            match field {
                Field::HistoricInput { name, .. } => bail!(
                    "unexpected historic input for first node `{first_node_name}` in `{}` for `{name}` in cycle context",
                    nodes.nodes[first_node_name].cycler_module
                ),
                Field::Input { name, .. } => bail!(
                    "unexpected optional input for first node `{first_node_name}` in `{}` for `{name}` in cycle context",
                    nodes.nodes[first_node_name].cycler_module
                ),
                Field::PerceptionInput { name, .. } => bail!(
                    "unexpected perception input for first node `{first_node_name}` in `{}` for `{name}` in cycle context",
                    nodes.nodes[first_node_name].cycler_module
                ),
                Field::RequiredInput { name, .. } => bail!(
                    "unexpected required input for first node `{first_node_name}` in `{}` for `{name}` in cycle context",
                    nodes.nodes[first_node_name].cycler_module
                ),
                _ => {}
            }
        }
    }

    let cyclers = get_cyclers(&cycler_instances, &nodes, &cycler_types);

    let cyclers_token_stream = generate_cyclers(&cyclers).wrap_err("failed to generate cyclers")?;
    let runtime_token_stream = generate_run(&cyclers);

    write_token_stream(
        "cyclers.rs",
        quote! {
            #cyclers_token_stream
            #runtime_token_stream
        },
    )
    .wrap_err("failed to write cyclers")?;

    Ok(())
}
