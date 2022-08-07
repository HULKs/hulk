use std::path::Path;

use anyhow::{anyhow, bail, Context};
use module_attributes2::Attribute;
use petgraph::{visit::EdgeRef, Graph};

use crate::{
    edge::Edge,
    get_cycler_instance_enum, get_module_implementation,
    node::Node,
    parse_file,
    parser::{get_cycler_instances, get_module, uses_from_items},
    queries::{
        find_additional_outputs_within_cycler, find_cycler_module_from_cycler_instance,
        find_main_outputs_within_cycler, find_parsed_rust_file_from_module_index,
        find_persistent_state_within_cycler, find_producing_module_from_read_edge_reference,
        find_uses_from_parsed_rust_file_index,
        iterate_producing_module_edges_from_main_outputs_struct_index,
    },
    to_absolute::ToAbsolute,
    walker::rust_file_paths_from,
};

pub fn source_graph_from<P>(parent_directory: P) -> anyhow::Result<Graph<Node, Edge>>
where
    P: AsRef<Path>,
{
    let mut graph = Graph::new();
    let configuration_index = graph.add_node(Node::Configuration);
    let hardware_interface_index = graph.add_node(Node::HardwareInterface);

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
            get_module(module_implementation).map_err(|error| {
                let start = error.span().start();
                anyhow!(
                    "Failed to parse module attributes: {error} at {}:{}:{}",
                    rust_file_path.display(),
                    start.line,
                    start.column
                )
            })
        });

        let parsed_rust_file_index = graph.add_node(Node::ParsedRustFile { file });
        graph.add_edge(rust_file_path_index, parsed_rust_file_index, Edge::Contains);

        if cycler_instances.is_some() && module.is_some() {
            bail!("Unexpected CyclerInstances and Module in a single file {rust_file_path:?}");
        }

        if let Some(cycler_instances) = cycler_instances {
            let cycler_module_directory = rust_file_path
                .parent()
                .expect("Expected at least the parent directory")
                .to_path_buf();
            let cycler_module_directory_name = cycler_module_directory
                .file_name()
                .ok_or_else(|| anyhow!("Failed to get file name of cycler module directory"))?
                .to_str()
                .ok_or_else(|| anyhow!("Failed to interpret cycler module name as Unicode"))?
                .to_string();
            let cycler_module_index = graph.add_node(Node::CyclerModule {
                module: cycler_module_directory_name.clone(),
                path: cycler_module_directory,
            });

            let main_outputs_index = graph.add_node(Node::Struct {
                name: "MainOutputs".to_string(),
                cycler_module: cycler_module_directory_name.clone(),
            });
            graph.add_edge(cycler_module_index, main_outputs_index, Edge::Contains);

            let additional_outputs_index = graph.add_node(Node::Struct {
                name: "AdditionalOutputs".to_string(),
                cycler_module: cycler_module_directory_name.clone(),
            });
            graph.add_edge(
                cycler_module_index,
                additional_outputs_index,
                Edge::Contains,
            );

            let persistent_state_index = graph.add_node(Node::Struct {
                name: "PersistentState".to_string(),
                cycler_module: cycler_module_directory_name,
            });
            graph.add_edge(cycler_module_index, persistent_state_index, Edge::Contains);

            for cycler_instance in cycler_instances {
                let cycler_instance_index = graph.add_node(Node::CyclerInstance {
                    instance: cycler_instance.to_string(),
                });
                graph.add_edge(
                    parsed_rust_file_index,
                    cycler_instance_index,
                    Edge::Contains,
                );
                graph.add_edge(cycler_module_index, cycler_instance_index, Edge::Contains);
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

    let cloned_graph = graph.clone();
    for (cycler_module_directory_index, cycler_module, cycler_module_directory) in cloned_graph
        .node_indices()
        .filter_map(|node_index| match &cloned_graph[node_index] {
            Node::CyclerModule { module, path } => Some((node_index, module, path)),
            _ => None,
        })
    {
        for rust_file_path_index in
            cloned_graph
                .node_indices()
                .filter_map(|node_index| match &cloned_graph[node_index] {
                    Node::RustFilePath { path } if path.starts_with(cycler_module_directory) => {
                        Some(node_index)
                    }
                    _ => None,
                })
        {
            graph.add_edge(
                cycler_module_directory_index,
                rust_file_path_index,
                Edge::Contains,
            );
        }

        for module_index in cloned_graph
            .node_indices()
            .filter_map(|node_index| match &cloned_graph[node_index] {
                Node::Module { module }
                    if module.attributes.iter().any(|attribute| match attribute {
                        Attribute::PerceptionModule {
                            cycler_module: cycler_module_of_attribute,
                        }
                        | Attribute::RealtimeModule {
                            cycler_module: cycler_module_of_attribute,
                        } => cycler_module_of_attribute == cycler_module,
                        _ => false,
                    }) =>
                {
                    Some(node_index)
                }
                _ => None,
            })
        {
            graph.add_edge(cycler_module_directory_index, module_index, Edge::Contains);
        }
    }

    let cloned_graph = graph.clone();
    for (module_index, module) in
        cloned_graph
            .node_indices()
            .filter_map(|node_index| match &cloned_graph[node_index] {
                Node::Module { module } => Some((node_index, module)),
                _ => None,
            })
    {
        let cycler_module = module
            .attributes
            .iter()
            .find_map(|attribute| match attribute {
                Attribute::PerceptionModule { cycler_module }
                | Attribute::RealtimeModule { cycler_module } => Some(cycler_module.to_string()),
                _ => None,
            })
            .ok_or_else(|| {
                anyhow!(
                    "Failed to find perception_module/realtime_module attribute of module {}",
                    module.module_identifier
                )
            })?;

        for attribute in module.attributes.iter() {
            match attribute {
                Attribute::AdditionalOutput { .. } => {
                    let additional_outputs_index = find_additional_outputs_within_cycler(&graph, &cycler_module)
                        .ok_or_else(|| anyhow!("Failed to find AdditionalOutputs in source graph of cycler module {cycler_module}"))?;
                    graph.add_edge(
                        module_index,
                        additional_outputs_index,
                        Edge::WritesTo {
                            attribute: attribute.clone(),
                        },
                    );
                }
                Attribute::HardwareInterface { .. } => {
                    graph.add_edge(
                        module_index,
                        hardware_interface_index,
                        Edge::ReadsFromOrWritesTo {
                            attribute: attribute.clone(),
                        },
                    );
                }
                Attribute::HistoricInput { .. } => {
                    let main_outputs_index = find_main_outputs_within_cycler(&graph, &cycler_module)
                        .ok_or_else(|| anyhow!("Failed to find MainOutputs in source graph of cycler module {cycler_module}"))?;
                    graph.add_edge(
                        module_index,
                        main_outputs_index,
                        Edge::ReadsFrom {
                            attribute: attribute.clone(),
                        },
                    );
                }
                Attribute::Input {
                    cycler_instance, ..
                } => {
                    let cycler_module = match cycler_instance {
                        Some(cycler_instance) => {
                            let cycler_module_index = find_cycler_module_from_cycler_instance(&graph, &cycler_instance.to_string())
                                .ok_or_else(|| anyhow!("Failed to find cycler module node in source graph of cycler instance {cycler_instance}"))?;
                            match &graph[cycler_module_index] {
                                Node::CyclerModule { module, path: _ } => module,
                                _ => panic!("Unexpected non-CyclerModule after successful search"),
                            }
                        }
                        None => &cycler_module,
                    };
                    let main_outputs_index = find_main_outputs_within_cycler(&graph, cycler_module)
                        .ok_or_else(|| anyhow!("Failed to find MainOutputs in source graph of cycler module {cycler_module}"))?;
                    graph.add_edge(
                        module_index,
                        main_outputs_index,
                        Edge::ReadsFrom {
                            attribute: attribute.clone(),
                        },
                    );
                }
                Attribute::MainOutput { .. } => {
                    let main_outputs_index = find_main_outputs_within_cycler(&graph, &cycler_module)
                        .ok_or_else(|| anyhow!("Failed to find MainOutputs in source graph of cycler module {cycler_module}"))?;
                    graph.add_edge(
                        module_index,
                        main_outputs_index,
                        Edge::WritesTo {
                            attribute: attribute.clone(),
                        },
                    );
                }
                Attribute::Parameter { .. } => {
                    graph.add_edge(
                        module_index,
                        configuration_index,
                        Edge::ReadsFrom {
                            attribute: attribute.clone(),
                        },
                    );
                }
                Attribute::PerceptionInput {
                    cycler_instance, ..
                } => {
                    let cycler_module_index = find_cycler_module_from_cycler_instance(&graph, &cycler_instance.to_string())
                        .ok_or_else(|| anyhow!("Failed to find cycler module node in source graph of cycler instance {cycler_instance}"))?;
                    let cycler_module = match &graph[cycler_module_index] {
                        Node::CyclerModule { module, path: _ } => module,
                        _ => panic!("Unexpected non-CyclerModule after successful search"),
                    };
                    let main_outputs_index = find_main_outputs_within_cycler(&graph, cycler_module)
                        .ok_or_else(|| anyhow!("Failed to find MainOutputs in source graph of cycler module {cycler_module}"))?;
                    graph.add_edge(
                        module_index,
                        main_outputs_index,
                        Edge::ReadsFrom {
                            attribute: attribute.clone(),
                        },
                    );
                }
                Attribute::PersistentState { .. } => {
                    let persistent_state_index = find_persistent_state_within_cycler(&graph, &cycler_module)
                        .ok_or_else(|| anyhow!("Failed to find PersistentState in source graph of cycler module {cycler_module}"))?;
                    graph.add_edge(
                        module_index,
                        persistent_state_index,
                        Edge::ReadsFromOrWritesTo {
                            attribute: attribute.clone(),
                        },
                    );
                }
                Attribute::PerceptionModule { .. } | Attribute::RealtimeModule { .. } => {}
            }
        }
    }

    let cloned_graph = graph.clone();
    for consuming_module_index in
        cloned_graph
            .node_indices()
            .filter_map(|node_index| match &cloned_graph[node_index] {
                Node::Module { .. } => Some(node_index),
                _ => None,
            })
    {
        for (edge_reference, attribute) in
            cloned_graph
                .edges(consuming_module_index)
                .filter_map(|edge_reference| match edge_reference.weight() {
                    Edge::ReadsFrom { attribute } => match attribute {
                        Attribute::HistoricInput { .. }
                        | Attribute::Input { .. }
                        | Attribute::PerceptionInput { .. } => Some((edge_reference, attribute)),
                        _ => None,
                    },
                    _ => None,
                })
        {
            let producing_module_index = find_producing_module_from_read_edge_reference(
                    &cloned_graph,
                    edge_reference,
                )
                .ok_or_else(|| {
                    let module_identifier = match &graph[consuming_module_index] {
                        Node::Module { module } => &module.module_identifier,
                        _ => panic!("consuming_module_index should refer to a Node::Module"),
                    };
                    anyhow!("Failed to find producing module in source graph for {attribute} in module {module_identifier}")
                })?;

            graph.add_edge(
                consuming_module_index,
                producing_module_index,
                Edge::ConsumesFrom {
                    attribute: attribute.clone(),
                },
            );
        }
    }

    let cloned_graph = graph.clone();
    for (struct_index, struct_name) in
        cloned_graph
            .node_indices()
            .filter_map(|node_index| match &cloned_graph[node_index] {
                Node::Struct { name, .. } => Some((node_index, name)),
                _ => None,
            })
    {
        match struct_name.as_str() {
            "MainOutputs" => {
                for (edge_reference, data_type, name) in
                    iterate_producing_module_edges_from_main_outputs_struct_index(
                        &cloned_graph,
                        struct_index,
                    )
                {
                    let parsed_rust_file_index = find_parsed_rust_file_from_module_index(&graph, edge_reference.source())
                        .ok_or_else(|| {
                            let module_identifier = match &graph[edge_reference.source()] {
                                Node::Module { module } => &module.module_identifier,
                                _ => panic!("edge_reference.source() should refer to a Node::Module"),
                            };
                            anyhow!("Failed to find ParsedRustFile in source graph for module {module_identifier}")
                        })?;
                    let uses_index =
                        find_uses_from_parsed_rust_file_index(&graph, parsed_rust_file_index)
                            .unwrap_or_else(|| {
                                let uses = match &graph[parsed_rust_file_index] {
                                    Node::ParsedRustFile { file } => uses_from_items(&file.items),
                                    _ => panic!(
                                "parsed_rust_file_index should refer to a Node::ParsedRustFile"
                            ),
                                };
                                let uses_index = graph.add_node(Node::Uses { uses });
                                graph.add_edge(parsed_rust_file_index, uses_index, Edge::Contains);
                                uses_index
                            });
                    let uses = match &graph[uses_index] {
                        Node::Uses { uses } => uses,
                        _ => panic!("uses_index should refer to a Node::Uses"),
                    };
                    let absolute_data_type = data_type.to_absolute(uses);
                    let struct_field_index = graph.add_node(Node::StructField {
                        data_type: absolute_data_type,
                    });
                    graph.add_edge(
                        struct_index,
                        struct_field_index,
                        Edge::ContainsField { name: name.clone() },
                    );
                }
            }
            "AdditionalOutputs" => {}
            "PersistentState" => {}
            _ => {}
        }
    }

    Ok(graph)
}
