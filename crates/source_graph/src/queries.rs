use std::collections::HashSet;

use anyhow::anyhow;
use convert_case::{Case, Casing};
use module_attributes2::{Attribute, Path};
use petgraph::{
    graph::EdgeReference,
    stable_graph::NodeIndex,
    visit::{Dfs, EdgeRef},
    Direction::{Incoming, Outgoing},
    Graph,
};
use syn::{Ident, Type};

use crate::{
    parser::{uses_from_items, Uses},
    Edge, Node,
};

pub fn find_struct_within_cycler(
    graph: &Graph<Node, Edge>,
    cycler_module: &str,
    struct_name: &str,
) -> Option<NodeIndex> {
    graph.edge_references().find_map(|edge_reference| {
        match (
            &graph[edge_reference.source()],
            edge_reference.weight(),
            &graph[edge_reference.target()],
        ) {
            (Node::CyclerModule { module, .. }, Edge::Contains, Node::Struct { name })
                if module == cycler_module && name == struct_name =>
            {
                Some(edge_reference.target())
            }
            _ => None,
        }
    })
}

pub fn find_cycler_module_from_cycler_instance(
    graph: &Graph<Node, Edge>,
    cycler_instance: &str,
) -> Option<NodeIndex> {
    graph
        .node_indices()
        .find(|node_index| match &graph[*node_index] {
            Node::CyclerModule { .. } => graph.neighbors(*node_index).any(|neighbor| match &graph
                [neighbor]
            {
                Node::CyclerInstance { instance } if instance == cycler_instance => true,
                _ => false,
            }),
            _ => false,
        })
}

pub fn find_producing_module_from_read_edge_reference(
    graph: &Graph<Node, Edge>,
    read_edge_reference: EdgeReference<Edge>,
) -> Option<NodeIndex> {
    let main_outputs_index = read_edge_reference.target();
    let read_edge_attribute = match read_edge_reference.weight() {
        Edge::ReadsFrom { attribute } => attribute,
        _ => return None,
    };
    let first_segment = match read_edge_attribute {
        Attribute::HistoricInput { path, .. }
        | Attribute::Input { path, .. }
        | Attribute::PerceptionInput { path, .. } => path.segments.first()?,
        _ => return None,
    };
    graph
        .edges_directed(main_outputs_index, Incoming)
        .find_map(|edge_reference| match edge_reference.weight() {
            Edge::WritesTo { attribute }
                if match attribute {
                    Attribute::MainOutput { name, .. } => first_segment == name,
                    _ => false,
                } =>
            {
                Some(edge_reference.source())
            }
            _ => None,
        })
}

pub fn iterate_producing_module_edges_from_main_outputs_struct_index(
    graph: &Graph<Node, Edge>,
    main_outputs_struct_index: NodeIndex,
) -> impl Iterator<Item = (EdgeReference<Edge>, &Type, &Ident)> {
    graph
        .edges_directed(main_outputs_struct_index, Incoming)
        .filter_map(|edge_reference| match edge_reference.weight() {
            Edge::WritesTo { attribute } => match attribute {
                Attribute::MainOutput { data_type, name } => {
                    Some((edge_reference, data_type, name))
                }
                _ => None,
            },
            _ => None,
        })
}

pub fn iterate_producing_module_edges_from_additional_outputs_struct_index(
    graph: &Graph<Node, Edge>,
    additional_outputs_struct_index: NodeIndex,
) -> impl Iterator<Item = (EdgeReference<Edge>, &Type, &Ident, &Path)> {
    graph
        .edges_directed(additional_outputs_struct_index, Incoming)
        .filter_map(|edge_reference| match edge_reference.weight() {
            Edge::WritesTo { attribute } => match attribute {
                Attribute::AdditionalOutput {
                    data_type,
                    name,
                    path,
                } => Some((edge_reference, data_type, name, path)),
                _ => None,
            },
            _ => None,
        })
}

pub fn iterate_producing_module_edges_from_persistent_state_struct_index(
    graph: &Graph<Node, Edge>,
    persistent_state_struct_index: NodeIndex,
) -> impl Iterator<Item = (EdgeReference<Edge>, &Type, &Ident, &Path)> {
    graph
        .edges_directed(persistent_state_struct_index, Incoming)
        .filter_map(|edge_reference| match edge_reference.weight() {
            Edge::ReadsFromOrWritesTo { attribute } => match attribute {
                Attribute::PersistentState {
                    data_type,
                    name,
                    path,
                } => Some((edge_reference, data_type, name, path)),
                _ => None,
            },
            _ => None,
        })
}

pub fn find_parsed_rust_file_from_module_index(
    graph: &Graph<Node, Edge>,
    module_index: NodeIndex,
) -> Option<NodeIndex> {
    graph
        .edges_directed(module_index, Incoming)
        .find_map(|edge_reference| match edge_reference.weight() {
            Edge::Contains
                if match graph[edge_reference.source()] {
                    Node::ParsedRustFile { .. } => true,
                    _ => false,
                } =>
            {
                Some(edge_reference.source())
            }
            _ => None,
        })
}

pub fn find_uses_from_parsed_rust_file_index(
    graph: &Graph<Node, Edge>,
    parsed_rust_file_index: NodeIndex,
) -> Option<NodeIndex> {
    graph
        .edges(parsed_rust_file_index)
        .find_map(|edge_reference| match edge_reference.weight() {
            Edge::Contains
                if match &graph[edge_reference.target()] {
                    Node::Uses { .. } => true,
                    _ => false,
                } =>
            {
                Some(edge_reference.target())
            }
            _ => None,
        })
}

pub fn store_and_get_uses_from_module_index(
    graph: &mut Graph<Node, Edge>,
    module_index: NodeIndex,
) -> anyhow::Result<&Uses> {
    let parsed_rust_file_index = find_parsed_rust_file_from_module_index(&graph, module_index)
        .ok_or_else(|| {
            let module_identifier = match &graph[module_index] {
                Node::Module { module } => &module.module_identifier,
                _ => panic!("edge_reference.source() should refer to a Node::Module"),
            };
            anyhow!("Failed to find ParsedRustFile in source graph for module {module_identifier}")
        })?;
    let uses_index = find_uses_from_parsed_rust_file_index(&graph, parsed_rust_file_index)
        .unwrap_or_else(|| {
            let uses = match &graph[parsed_rust_file_index] {
                Node::ParsedRustFile { file } => uses_from_items(&file.items),
                _ => panic!("parsed_rust_file_index should refer to a Node::ParsedRustFile"),
            };
            let uses_index = graph.add_node(Node::Uses { uses });
            graph.add_edge(parsed_rust_file_index, uses_index, Edge::Contains);
            uses_index
        });
    match &graph[uses_index] {
        Node::Uses { uses } => Ok(uses),
        _ => panic!("uses_index should refer to a Node::Uses"),
    }
}

pub fn remove_tree(graph: &mut Graph<Node, Edge>, removal_root_index: NodeIndex) {
    let mut node_indices_to_remove = HashSet::new();
    let mut depth_first_search = Dfs::new(&*graph, removal_root_index);
    while let Some(node_index) = depth_first_search.next(&*graph) {
        node_indices_to_remove.insert(node_index);
    }
    graph.retain_nodes(|_graph, node_index| !node_indices_to_remove.contains(&node_index));
}

pub fn add_path_to_struct_hierarchy(
    graph: &mut Graph<Node, Edge>,
    root_struct_index: NodeIndex,
    root_struct_name: String,
    data_type: Type,
    path: &Path,
) {
    let mut current_node_index = root_struct_index;
    let mut current_struct_name = root_struct_name;
    for segment in path.segments.iter() {
        match &graph[current_node_index] {
            Node::StructField { .. } => break,
            _ => {}
        }
        current_struct_name += &segment.to_string().to_case(Case::Pascal);
        current_node_index = match graph.edges(current_node_index).find(|edge_reference| {
            match edge_reference.weight() {
                Edge::ContainsField { name } if name == segment => true,
                _ => false,
            }
        }) {
            Some(edge_reference) => edge_reference.target(),
            None => {
                let next_node_index = graph.add_node(Node::Struct {
                    name: current_struct_name.clone(),
                });
                graph.add_edge(
                    current_node_index,
                    next_node_index,
                    Edge::ContainsField {
                        name: segment.clone(),
                    },
                );
                next_node_index
            }
        };
    }
    while let Some(neighbor_edge_index) = graph.first_edge(current_node_index, Outgoing) {
        let (_, neighbor_index) = graph
            .edge_endpoints(neighbor_edge_index)
            .expect("Found edge must have endpoints");
        remove_tree(graph, neighbor_index);
    }
    graph[current_node_index] = Node::StructField { data_type };
}
