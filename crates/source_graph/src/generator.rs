use std::path::Path;

use anyhow::{anyhow, bail, Context};
use module_attributes2::Attribute;
use petgraph::{stable_graph::NodeIndex, visit::EdgeRef, Graph};

use crate::{
    edge::Edge,
    get_cycler_instance_enum, get_module_implementation,
    node::Node,
    parse_file,
    parser::{get_cycler_instances, get_module},
    queries::{
        add_path_to_struct_hierarchy, find_cycler_module_from_cycler_instance,
        find_producing_module_from_read_edge_reference, find_struct_within_cycler,
        iterate_cycler_modules, iterate_modules, iterate_modules_with_matching_cycler_module,
        iterate_producing_module_edges_from_additional_outputs_struct_index,
        iterate_producing_module_edges_from_configuration_struct_index,
        iterate_producing_module_edges_from_main_outputs_struct_index,
        iterate_producing_module_edges_from_persistent_state_struct_index,
        iterate_read_edge_references_from_module_index, iterate_rust_file_paths,
        iterate_rust_file_paths_starting_with_path, iterate_structs,
        store_and_get_uses_from_module_index,
    },
    to_absolute::ToAbsolute,
    walker::rust_file_paths_from,
};

pub fn source_graph_from<P>(parent_directory: P) -> anyhow::Result<Graph<Node, Edge>>
where
    P: AsRef<Path>,
{
    let mut graph = Graph::new();
    let (configuration_index, hardware_interface_index) = generate_initial_root_structs(&mut graph);
    generate_rust_file_paths(&mut graph, parent_directory);
    generate_modules_and_cycler_instances_and_root_structs(&mut graph)
        .context("Failed to generate modules and cycler instances")?;
    connect_nodes_with_corresponding_cycler_modules(&mut graph);
    generate_attribute_edges(&mut graph, configuration_index, hardware_interface_index)
        .context("Failed to generate attribute edges")?;
    generate_consumer_producer_edges(&mut graph)
        .context("Failed to generate consumer/producer edges")?;
    generate_struct_hierarchy(&mut graph).context("Failed to generate struct hierarchy")?;
    Ok(graph)
}

fn generate_initial_root_structs(graph: &mut Graph<Node, Edge>) -> (NodeIndex, NodeIndex) {
    let configuration_index = graph.add_node(Node::Struct {
        name: "Configuration".to_string(),
    });
    let hardware_interface_index = graph.add_node(Node::HardwareInterface);
    (configuration_index, hardware_interface_index)
}

fn generate_rust_file_paths<P>(graph: &mut Graph<Node, Edge>, parent_directory: P)
where
    P: AsRef<Path>,
{
    for rust_file_path in rust_file_paths_from(parent_directory) {
        graph.add_node(Node::RustFilePath {
            path: rust_file_path,
        });
    }
}

fn generate_modules_and_cycler_instances_and_root_structs(
    graph: &mut Graph<Node, Edge>,
) -> anyhow::Result<()> {
    let cloned_graph = graph.clone();
    for (rust_file_path_index, rust_file_path) in iterate_rust_file_paths(&cloned_graph) {
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
            });
            graph.add_edge(cycler_module_index, main_outputs_index, Edge::Contains);

            let additional_outputs_index = graph.add_node(Node::Struct {
                name: "AdditionalOutputs".to_string(),
            });
            graph.add_edge(
                cycler_module_index,
                additional_outputs_index,
                Edge::Contains,
            );

            let persistent_state_index = graph.add_node(Node::Struct {
                name: "PersistentState".to_string(),
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
    Ok(())
}

fn connect_nodes_with_corresponding_cycler_modules(graph: &mut Graph<Node, Edge>) {
    let cloned_graph = graph.clone();
    for (cycler_module_directory_index, cycler_module, cycler_module_directory) in
        iterate_cycler_modules(&cloned_graph)
    {
        for rust_file_path_index in
            iterate_rust_file_paths_starting_with_path(&cloned_graph, cycler_module_directory)
        {
            graph.add_edge(
                cycler_module_directory_index,
                rust_file_path_index,
                Edge::Contains,
            );
        }

        for module_index in
            iterate_modules_with_matching_cycler_module(&cloned_graph, cycler_module)
        {
            graph.add_edge(cycler_module_directory_index, module_index, Edge::Contains);
        }
    }
}

fn generate_attribute_edges(
    graph: &mut Graph<Node, Edge>,
    configuration_index: NodeIndex,
    hardware_interface_index: NodeIndex,
) -> anyhow::Result<()> {
    let cloned_graph = graph.clone();
    for (module_index, module) in iterate_modules(&cloned_graph) {
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
                    let additional_outputs_index = find_struct_within_cycler(&graph, &cycler_module, "AdditionalOutputs")
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
                    let main_outputs_index = find_struct_within_cycler(&graph, &cycler_module, "MainOutputs")
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
                    let main_outputs_index = find_struct_within_cycler(&graph, &cycler_module, "MainOutputs")
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
                    let main_outputs_index = find_struct_within_cycler(&graph, &cycler_module, "MainOutputs")
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
                    let main_outputs_index = find_struct_within_cycler(&graph, &cycler_module, "MainOutputs")
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
                    let persistent_state_index = find_struct_within_cycler(&graph, &cycler_module, "PersistentState")
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
    Ok(())
}

fn generate_consumer_producer_edges(graph: &mut Graph<Node, Edge>) -> anyhow::Result<()> {
    let cloned_graph = graph.clone();
    for (consuming_module_index, consuming_module) in iterate_modules(&cloned_graph) {
        for (edge_reference, attribute) in
            iterate_read_edge_references_from_module_index(&cloned_graph, consuming_module_index)
        {
            let producing_module_index =
                find_producing_module_from_read_edge_reference(&cloned_graph, edge_reference)
                    .ok_or_else(|| {
                        anyhow!(
                    "Failed to find producing module in source graph for {attribute} in module {}",
                    consuming_module.module_identifier
                )
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
    Ok(())
}

fn generate_struct_hierarchy(graph: &mut Graph<Node, Edge>) -> anyhow::Result<()> {
    let cloned_graph = graph.clone();
    for (root_struct_index, root_struct_name) in iterate_structs(&cloned_graph) {
        match root_struct_name.as_str() {
            "Configuration" => {
                for (edge_reference, data_type, _name, path) in
                    iterate_producing_module_edges_from_configuration_struct_index(
                        &cloned_graph,
                        root_struct_index,
                    )
                {
                    let uses =
                        store_and_get_uses_from_module_index(graph, edge_reference.source())?;
                    let absolute_data_type = data_type.to_absolute(uses);
                    add_path_to_struct_hierarchy(
                        graph,
                        root_struct_index,
                        root_struct_name.clone(),
                        absolute_data_type,
                        path,
                    );
                }
            }
            "MainOutputs" => {
                for (edge_reference, data_type, name) in
                    iterate_producing_module_edges_from_main_outputs_struct_index(
                        &cloned_graph,
                        root_struct_index,
                    )
                {
                    let uses =
                        store_and_get_uses_from_module_index(graph, edge_reference.source())?;
                    let absolute_data_type = data_type.to_absolute(uses);
                    let struct_field_index = graph.add_node(Node::StructField {
                        data_type: absolute_data_type,
                    });
                    graph.add_edge(
                        root_struct_index,
                        struct_field_index,
                        Edge::ContainsField { name: name.clone() },
                    );
                }
            }
            "AdditionalOutputs" => {
                for (edge_reference, data_type, _name, path) in
                    iterate_producing_module_edges_from_additional_outputs_struct_index(
                        &cloned_graph,
                        root_struct_index,
                    )
                {
                    let uses =
                        store_and_get_uses_from_module_index(graph, edge_reference.source())?;
                    let absolute_data_type = data_type.to_absolute(uses);
                    add_path_to_struct_hierarchy(
                        graph,
                        root_struct_index,
                        root_struct_name.clone(),
                        absolute_data_type,
                        path,
                    );
                }
            }
            "PersistentState" => {
                for (edge_reference, data_type, _name, path) in
                    iterate_producing_module_edges_from_persistent_state_struct_index(
                        &cloned_graph,
                        root_struct_index,
                    )
                {
                    let uses =
                        store_and_get_uses_from_module_index(graph, edge_reference.source())?;
                    let absolute_data_type = data_type.to_absolute(uses);
                    add_path_to_struct_hierarchy(
                        graph,
                        root_struct_index,
                        root_struct_name.clone(),
                        absolute_data_type,
                        path,
                    );
                }
            }
            _ => {}
        }
    }
    Ok(())
}
