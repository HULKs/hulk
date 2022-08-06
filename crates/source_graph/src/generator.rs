use std::path::Path;

use anyhow::{bail, Context};
use petgraph::Graph;

use crate::{
    edge::Edge,
    get_cycler_instance_enum, get_module_implementation,
    node::Node,
    parse_file,
    parser::{get_cycler_instances, get_module},
    walker::rust_file_paths_from,
};

pub fn source_graph_from<P>(parent_directory: P) -> anyhow::Result<Graph<Node, Edge>>
where
    P: AsRef<Path>,
{
    let mut graph = Graph::new();
    for rust_file_path in rust_file_paths_from(parent_directory) {
        graph.add_node(Node::RustFilePath {
            path: rust_file_path,
        });
    }

    let cloned_graph = graph.clone();
    for (rust_file_path_index, rust_file_path) in
        cloned_graph
            .node_indices()
            .filter_map(|node_index| match &cloned_graph[node_index] {
                Node::RustFilePath { path } => Some((node_index, path.clone())),
                _ => None,
            })
    {
        let file = parse_file(&rust_file_path)
            .with_context(|| format!("Failed to parse file {rust_file_path:?}"))?;
        let cycler_instances = get_cycler_instance_enum(&file)
            .map(|cycler_instance_enum| get_cycler_instances(cycler_instance_enum));
        let module = get_module_implementation(&file).map(|module_implementation| {
            get_module(module_implementation).with_context(|| {
                format!("Failed to parse module attributes from {rust_file_path:?}")
            })
        });

        let parsed_rust_file_index = graph.add_node(Node::ParsedRustFile { file });
        graph.add_edge(rust_file_path_index, parsed_rust_file_index, Edge::Contains);

        if cycler_instances.is_some() && module.is_some() {
            bail!("Unexpected CyclerInstances and Module in a single file {rust_file_path:?}");
        }

        if let Some(cycler_instances) = cycler_instances {
            for cycler_instance in cycler_instances {
                let cycler_instance_index = graph.add_node(Node::CyclerInstance {
                    instance: cycler_instance,
                });
                graph.add_edge(
                    parsed_rust_file_index,
                    cycler_instance_index,
                    Edge::Contains,
                );
            }
        }

        if let Some(module) = module {
            let module = module?;
            let cycler_instance_index = graph.add_node(Node::Module { module });
            graph.add_edge(
                parsed_rust_file_index,
                cycler_instance_index,
                Edge::Contains,
            );
        }
    }

    Ok(graph)
}
